use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, Error};
use cln_plugin::Plugin;
use log::{debug, warn};
use serde_json::json;
use tokio::time;

use crate::{
    config::PluginState, listdatastore, listinvoices, make_rpc_path, Hodlstate, CLTV_HODL,
    PLUGIN_NAME,
};

pub async fn htlc_handler(
    plugin: Plugin<PluginState>,
    v: serde_json::Value,
) -> Result<serde_json::Value, Error> {
    if let Some(htlc) = v.get("htlc") {
        if let Some(pay_hash) = htlc
            .get("payment_hash")
            .and_then(|pay_hash| pay_hash.as_str())
        {
            let rpc_path = make_rpc_path(&plugin);
            let mut invoice = None;
            let cltv_delta = plugin.state().config.lock().clone().cltv_delta.1 as u64;
            let cltv_expiry = match htlc.get("cltv_expiry") {
                Some(ce) => ce.as_u64().unwrap(),
                None => return Err(anyhow!("expiry not found! payment_hash: {}", pay_hash)),
            };
            loop {
                {
                    match listdatastore(
                        &rpc_path,
                        Some(vec![PLUGIN_NAME.to_string(), pay_hash.to_string()]),
                    )
                    .await
                    {
                        Ok(resp) => {
                            if resp.datastore.len() != 1 {
                                return Err(anyhow!(
                                    "wrong amount of results found for payment_hash: {} {:?}",
                                    pay_hash,
                                    resp.datastore
                                ));
                            } else {
                                if invoice.is_none() {
                                    invoice = Some(
                                        listinvoices(&rpc_path, None, Some(pay_hash.to_string()))
                                            .await?
                                            .invoices
                                            .first()
                                            .ok_or(anyhow!("invoice not found"))?
                                            .clone(),
                                    );
                                }

                                let now = SystemTime::now()
                                    .duration_since(UNIX_EPOCH)
                                    .unwrap()
                                    .as_secs();

                                if invoice.as_ref().unwrap().expires_at <= now {
                                    warn!(
                                        "hodling invoice with payment_hash: {} expired, rejecting!",
                                        pay_hash
                                    );
                                    return Ok(json!({"result": "fail"}));
                                }

                                if cltv_expiry - cltv_delta
                                    <= plugin.state().blockheight.lock().clone() + CLTV_HODL as u64
                                {
                                    warn!(
                                        "htlc timed out for payment_hash: {}, rejecting!",
                                        pay_hash
                                    );
                                    return Ok(json!({"result": "fail"}));
                                }

                                let hodlstate = Hodlstate::from_str(
                                    resp.datastore.first().unwrap().string.as_ref().unwrap(),
                                )
                                .unwrap();
                                // debug!(
                                //     "htlc_handler: payment_hash: {} hodlstate: {:?}",
                                //     pay_hash, resp,
                                // );
                                match hodlstate {
                                    Hodlstate::Hodl => {
                                        debug!("hodling invoice with payment_hash: {}", pay_hash);
                                    }
                                    Hodlstate::Accept => {
                                        debug!("accepted invoice with payment_hash: {}", pay_hash);
                                        return Ok(json!({"result": "continue"}));
                                    }
                                    Hodlstate::Reject => {
                                        debug!("rejected invoice with payment_hash: {}", pay_hash);
                                        return Ok(json!({"result": "fail"}));
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            debug!(
                                "{} not our invoice: payment_hash: {}",
                                e.to_string(),
                                pay_hash
                            );
                            return Ok(json!({"result": "continue"}));
                        }
                    }
                }
                time::sleep(Duration::from_secs(2)).await;
            }
        }
    }
    Ok(json!({"result": "continue"}))
}

pub async fn block_added(plugin: Plugin<PluginState>, v: serde_json::Value) -> Result<(), Error> {
    match v.get("block") {
        Some(block) => match block.get("height") {
            Some(h) => *plugin.state().blockheight.lock() = h.as_u64().unwrap(),
            None => return Err(anyhow!("could not find height for block")),
        },
        None => return Err(anyhow!("could not read block notification")),
    };
    Ok(())
}
