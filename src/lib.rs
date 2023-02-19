use std::path::{Path, PathBuf};

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
use log::debug;
use serde_json::json;

pub mod hooks;

pub const PLUGIN_NAME: &str = "hodlvoice";
#[derive(Clone)]
pub struct PluginState {}
impl PluginState {
    pub fn new() -> PluginState {
        PluginState {}
    }
}

#[derive(Debug, Clone)]
pub enum Hodlstate {
    Hodl,
    Reject,
    Accept,
}

pub async fn hodlvoice(
    plugin: Plugin<PluginState>,
    args: serde_json::Value,
) -> Result<serde_json::Value, Error> {
    let rpc_path = &make_rpc_path(&plugin);
    let label;
    match args {
        serde_json::Value::Array(a) => {
            if a.len() != 1 {
                return Err(anyhow!("Please provide exactly one `label`"));
            } else {
                match a.first().unwrap() {
                    serde_json::Value::String(i) => {
                        label = i.clone();
                    }
                    _ => return Err(anyhow!("invalid string for label")),
                }
            }
        }
        _ => return Err(anyhow!("invalid arguments")),
    };
    let inv = invoice(
        rpc_path,
        Amount::from_msat(1000),
        "cock".to_string(),
        label.clone(),
    )
    .await?;
    debug!("{}", inv.bolt11);
    let datastore = datastore(
        rpc_path,
        vec![PLUGIN_NAME.to_string(), label],
        Some("Hodl".to_string()),
        None,
        Some(DatastoreMode::MUST_CREATE),
        None,
    )
    .await?;
    let listdata = listdatastore(rpc_path, None).await?;
    debug!("{:?}", listdata);
    Ok(json!({"result": "success"}))
}

pub async fn hodlvoiceaccept(
    plugin: Plugin<PluginState>,
    args: serde_json::Value,
) -> Result<serde_json::Value, Error> {
    debug!("test acc0");
    let rpc_path = &make_rpc_path(&plugin);
    match args {
        serde_json::Value::Array(a) => {
            if a.len() != 1 {
                return Err(anyhow!("Please provide exactly one `label`"));
            } else {
                match a.first().unwrap() {
                    serde_json::Value::String(i) => {
                        debug!("test acc1");
                        datastore(
                            rpc_path,
                            vec![PLUGIN_NAME.to_string(), i.clone()],
                            Some("Accept".to_string()),
                            None,
                            Some(DatastoreMode::MUST_REPLACE),
                            None,
                        )
                        .await?;
                        debug!("test acc2");
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
                return Err(anyhow!("Please provide exactly one `label`"));
            } else {
                match a.first().unwrap() {
                    serde_json::Value::String(i) => {
                        datastore(
                            rpc_path,
                            vec![PLUGIN_NAME.to_string(), i.clone()],
                            Some("Reject".to_string()),
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
) -> Result<InvoiceResponse, Error> {
    let mut rpc = ClnRpc::new(&rpc_path).await?;
    let invoice_request = rpc
        .call(Request::Invoice(InvoiceRequest {
            amount_msat: AmountOrAny::Amount(amount_msat),
            description,
            label,
            expiry: None,
            fallbacks: None,
            preimage: None,
            exposeprivatechannels: None,
            cltv: Some(50),
            deschashonly: None,
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
