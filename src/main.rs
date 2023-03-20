use anyhow::anyhow;
use cln_plugin::Builder;
use hodlvoice::{
    config::{read_config, PluginState},
    hodlvoiceaccept, hodlvoiceadd, hodlvoicereject,
    hooks::block_added,
    hooks::htlc_handler,
    PLUGIN_NAME,
};
use log::info;
use tokio::{self};
#[cfg(all(not(windows), not(target_env = "musl")))]
#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    std::env::set_var("CLN_PLUGIN_LOG", "trace");
    let state = PluginState::new();
    let confplugin;
    match Builder::new(tokio::io::stdin(), tokio::io::stdout())
        .rpcmethod(
            &(PLUGIN_NAME.to_string() + "-add"),
            "add hold-invoice",
            hodlvoiceadd,
        )
        .rpcmethod(
            &(PLUGIN_NAME.to_string() + "-accept"),
            "accept hold-invoice",
            hodlvoiceaccept,
        )
        .rpcmethod(
            &(PLUGIN_NAME.to_string() + "-reject"),
            "reject hold-invoice",
            hodlvoicereject,
        )
        .hook("htlc_accepted", htlc_handler)
        .subscribe("block_added", block_added)
        .dynamic()
        .configure()
        .await?
    {
        Some(plugin) => {
            info!("read config");
            match read_config(&plugin, state.clone()).await {
                Ok(()) => &(),
                Err(e) => return plugin.disable(format!("{}", e).as_str()).await,
            };
            confplugin = plugin;
        }
        None => return Err(anyhow!("Error configuring the plugin!")),
    };
    if let Ok(plugin) = confplugin.start(state).await {
        plugin.join().await
    } else {
        Err(anyhow!("Error starting the plugin!"))
    }
}
