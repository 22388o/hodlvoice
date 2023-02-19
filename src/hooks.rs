use std::{path::PathBuf, thread, time::Duration};

use anyhow::{anyhow, Error};
use cln_plugin::Plugin;
use log::debug;
use serde_json::json;
use tokio::time;

use crate::{listdatastore, make_rpc_path, Hodlstate, PluginState, PLUGIN_NAME};

pub async fn invoice_payment_handler(
    plugin: Plugin<PluginState>,
    v: serde_json::Value,
) -> Result<serde_json::Value, Error> {
    let rpc_path = make_rpc_path(&plugin);
    let label_str;
    match v.get("payment").cloned() {
        Some(paym) => match paym.get("label").cloned() {
            Some(label) => {
                label_str = label.as_str().unwrap().to_string();
            }
            None => {
                debug!("label not found in hook");
                return Ok(json!({"result": "continue"}));
            }
        },
        None => {
            debug!("malformed hook");
            return Ok(json!({"result": "continue"}));
        }
    }
    loop {
        {
            match listdatastore(
                &rpc_path,
                Some(vec![PLUGIN_NAME.to_string(), label_str.to_string()]),
            )
            .await
            {
                Ok(pays) => {
                    if pays.datastore.len() != 1 {
                        return Err(anyhow!(
                            "wrong amount of results found for label: {}",
                            label_str
                        ));
                    } else {
                        let hodlstate = pays.datastore.first().unwrap().string.as_ref().unwrap();
                        debug!(
                            "invoice_payment_handler: label: {} hodlstate: {:?}",
                            label_str, pays,
                        );
                        match hodlstate {
                            op if op.eq("Hodl") => {
                                debug!("hodling invoice with label: {}", label_str);
                            }
                            op if op.eq("Accept") => {
                                debug!("accepted invoice with label: {}", label_str);
                                return Ok(json!({"result": "continue"}));
                            }
                            op if op.eq("Reject") => {
                                debug!("rejected invoice with label: {}", label_str);
                                return Ok(json!({"result": "reject"}));
                            }
                            _ => {
                                return Err(anyhow!("unknown hodlstate: {}", hodlstate));
                            }
                        }
                    }
                }
                Err(e) => {
                    debug!("{} not our invoice: label: {}", e.to_string(), label_str);
                    return Ok(json!({"result": "continue"}));
                }
            }
        }
        time::sleep(Duration::from_secs(3)).await;
    }
}
