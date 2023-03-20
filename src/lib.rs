use std::{
    fmt,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Error};
use cln_plugin::Plugin;
use cln_rpc::{
    model::{
        DatastoreMode, DatastoreRequest, DatastoreResponse, InvoiceRequest, InvoiceResponse,
        ListdatastoreRequest, ListdatastoreResponse, ListinvoicesRequest, ListinvoicesResponse,
    },
    primitives::{Amount, AmountOrAny},
    ClnRpc, Request, Response,
};
use config::PluginState;
use serde_json::json;

pub mod config;
pub mod hooks;

pub const PLUGIN_NAME: &str = "hodlvoice";
pub const CLTV_HODL: u32 = 200;

#[derive(Debug, Clone)]
pub enum Hodlstate {
    Hodl,
    Reject,
    Accept,
}
impl Hodlstate {
    pub fn to_string(&self) -> String {
        match self {
            Hodlstate::Hodl => "hodl".to_string(),
            Hodlstate::Reject => "reject".to_string(),
            Hodlstate::Accept => "accept".to_string(),
        }
    }
}
impl Hodlstate {
    pub fn from_str(s: &str) -> Option<Hodlstate> {
        match s.to_lowercase().as_str() {
            "hodl" => Some(Hodlstate::Hodl),
            "reject" => Some(Hodlstate::Reject),
            "accept" => Some(Hodlstate::Accept),
            _ => None,
        }
    }
}
impl fmt::Display for Hodlstate {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Hodlstate::Hodl => write!(f, "Hodl"),
            Hodlstate::Reject => write!(f, "Reject"),
            Hodlstate::Accept => write!(f, "Accept"),
        }
    }
}

pub async fn hodlvoiceadd(
    plugin: Plugin<PluginState>,
    args: serde_json::Value,
) -> Result<serde_json::Value, Error> {
    let rpc_path = &make_rpc_path(&plugin);
    let valid_keys = vec![
        "amount_msat",
        "description",
        "label",
        "expiry",
        "fallbacks",
        "preimage",
        "exposeprivatechannels",
        "deschashonly",
    ];

    let config = plugin.state().config.lock().clone();

    let my_invoice;
    match args {
        serde_json::Value::Object(ar) => {
            for k in ar.keys() {
                if !valid_keys.contains(&k.as_str()) {
                    return Err(anyhow!("Invalid argument: {}", k));
                }
            }

            let amount_msat;
            match ar.get("amount_msat") {
                Some(amt) => {
                    amount_msat = Amount::from_msat(
                        amt.as_u64()
                            .ok_or(anyhow!("invalid string for amount_msat"))?,
                    )
                }
                None => return Err(anyhow!("Missing amount_msat")),
            }

            let description;
            match ar.get("description") {
                Some(desc) => {
                    description = desc
                        .as_str()
                        .ok_or(anyhow!("invalid string for description"))?
                        .to_string();
                }
                None => return Err(anyhow!("Missing description")),
            }

            let label;
            match ar.get("label") {
                Some(lbl) => {
                    label = lbl
                        .as_str()
                        .ok_or(anyhow!("invalid string for label"))?
                        .to_string();
                }
                None => return Err(anyhow!("Missing label")),
            }

            let expiry = match ar.get("expiry") {
                Some(exp) => Some(exp.as_u64().ok_or(anyhow!("expiry must be an integer"))?),
                None => None,
            };

            let fallbacks = match ar.get("fallbacks") {
                Some(o) => {
                    let mut fb_vec: Vec<String> = Vec::new();
                    for fb in o
                        .as_array()
                        .ok_or(anyhow!("fallbacks must be an array of Strings"))?
                    {
                        fb_vec.push(
                            fb.as_str()
                                .ok_or(anyhow!("invalid input for fallback string: {}", fb))?
                                .to_string(),
                        )
                    }
                    Some(fb_vec)
                }
                None => None,
            };

            let preimage = match ar.get("preimage") {
                Some(prei) => Some(
                    prei.as_str()
                        .ok_or(anyhow!("invalid string for preimage"))?
                        .to_string(),
                ),
                None => None,
            };

            let exposeprivatechannels = match ar.get("exposeprivatechannels") {
                Some(h) => Some(
                    h.as_bool()
                        .ok_or(anyhow!("exposeprivatechannels must be a bool"))?,
                ),
                None => None,
            };

            let deschashonly = match ar.get("deschashonly") {
                Some(dp) => Some(dp.as_bool().ok_or(anyhow!("deschashonly must be a bool"))?),
                None => None,
            };

            my_invoice = invoice(
                rpc_path,
                amount_msat,
                description,
                label,
                expiry,
                fallbacks,
                preimage,
                exposeprivatechannels,
                Some(CLTV_HODL + config.cltv_delta.1 as u32),
                deschashonly,
            )
            .await?;
        }
        other => return Err(anyhow!("Invalid arguments: {}", other.to_string())),
    }

    let _datastore = datastore(
        rpc_path,
        vec![PLUGIN_NAME.to_string(), my_invoice.payment_hash.to_string()],
        Some(Hodlstate::Hodl.to_string()),
        None,
        Some(DatastoreMode::MUST_CREATE),
        None,
    )
    .await?;

    Ok(json!(my_invoice))
}

pub async fn hodlvoiceaccept(
    plugin: Plugin<PluginState>,
    args: serde_json::Value,
) -> Result<serde_json::Value, Error> {
    let rpc_path = &make_rpc_path(&plugin);
    match args {
        serde_json::Value::Array(a) => {
            if a.len() != 1 {
                return Err(anyhow!("Please provide exactly one `payment_hash`"));
            } else {
                match a.first().unwrap() {
                    serde_json::Value::String(i) => {
                        datastore(
                            rpc_path,
                            vec![PLUGIN_NAME.to_string(), i.clone()],
                            Some(Hodlstate::Accept.to_string()),
                            None,
                            Some(DatastoreMode::MUST_REPLACE),
                            None,
                        )
                        .await?;
                    }
                    _ => return Err(anyhow!("invalid string for accepting hold-invoice")),
                };
            }
        }
        _ => return Err(anyhow!("invalid arguments")),
    };

    Ok(json!({"result": "success"}))
}

pub async fn hodlvoicereject(
    plugin: Plugin<PluginState>,
    args: serde_json::Value,
) -> Result<serde_json::Value, Error> {
    let rpc_path = &make_rpc_path(&plugin);
    match args {
        serde_json::Value::Array(a) => {
            if a.len() != 1 {
                return Err(anyhow!("Please provide exactly one `payment_hash`"));
            } else {
                match a.first().unwrap() {
                    serde_json::Value::String(i) => {
                        datastore(
                            rpc_path,
                            vec![PLUGIN_NAME.to_string(), i.clone()],
                            Some(Hodlstate::Reject.to_string()),
                            None,
                            Some(DatastoreMode::MUST_REPLACE),
                            None,
                        )
                        .await?;
                    }
                    _ => return Err(anyhow!("invalid string for rejecting hold-invoice")),
                };
            }
        }
        _ => return Err(anyhow!("invalid arguments")),
    };

    Ok(json!({"result": "success"}))
}

pub async fn invoice(
    rpc_path: &PathBuf,
    amount_msat: Amount,
    description: String,
    label: String,
    expiry: Option<u64>,
    fallbacks: Option<Vec<String>>,
    preimage: Option<String>,
    exposeprivatechannels: Option<bool>,
    cltv: Option<u32>,
    deschashonly: Option<bool>,
) -> Result<InvoiceResponse, Error> {
    let mut rpc = ClnRpc::new(&rpc_path).await?;
    let invoice_request = rpc
        .call(Request::Invoice(InvoiceRequest {
            amount_msat: AmountOrAny::Amount(amount_msat),
            description,
            label,
            expiry,
            fallbacks,
            preimage,
            exposeprivatechannels,
            cltv,
            deschashonly,
        }))
        .await
        .map_err(|e| anyhow!("Error calling invoice: {:?}", e))?;
    match invoice_request {
        Response::Invoice(info) => Ok(info),
        e => Err(anyhow!("Unexpected result in invoice: {:?}", e)),
    }
}

pub async fn listinvoices(
    rpc_path: &PathBuf,
    label: Option<String>,
    payment_hash: Option<String>,
) -> Result<ListinvoicesResponse, Error> {
    let mut rpc = ClnRpc::new(&rpc_path).await?;
    let invoice_request = rpc
        .call(Request::ListInvoices(ListinvoicesRequest {
            label,
            invstring: None,
            payment_hash,
            offer_id: None,
        }))
        .await
        .map_err(|e| anyhow!("Error calling listinvoices: {:?}", e))?;
    match invoice_request {
        Response::ListInvoices(info) => Ok(info),
        e => Err(anyhow!("Unexpected result in listinvoices: {:?}", e)),
    }
}

pub async fn datastore(
    rpc_path: &PathBuf,
    key: Vec<String>,
    string: Option<String>,
    hex: Option<String>,
    mode: Option<DatastoreMode>,
    generation: Option<u64>,
) -> Result<DatastoreResponse, Error> {
    let mut rpc = ClnRpc::new(&rpc_path).await?;
    let datastore_request = rpc
        .call(Request::Datastore(DatastoreRequest {
            key,
            string,
            hex,
            mode,
            generation,
        }))
        .await
        .map_err(|e| anyhow!("Error calling datastore: {:?}", e))?;
    match datastore_request {
        Response::Datastore(info) => Ok(info),
        e => Err(anyhow!("Unexpected result in datastore: {:?}", e)),
    }
}

pub async fn listdatastore(
    rpc_path: &PathBuf,
    key: Option<Vec<String>>,
) -> Result<ListdatastoreResponse, Error> {
    let mut rpc = ClnRpc::new(&rpc_path).await?;
    let datastore_request = rpc
        .call(Request::ListDatastore(ListdatastoreRequest { key }))
        .await
        .map_err(|e| anyhow!("Error calling listdatastore: {:?}", e))?;
    match datastore_request {
        Response::ListDatastore(info) => Ok(info),
        e => Err(anyhow!("Unexpected result in listdatastore: {:?}", e)),
    }
}

pub fn make_rpc_path(plugin: &Plugin<PluginState>) -> PathBuf {
    Path::new(&plugin.configuration().lightning_dir).join(plugin.configuration().rpc_file)
}
