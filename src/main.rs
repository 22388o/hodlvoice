use anyhow::anyhow;
use cln_plugin::Builder;
use hodlvoice::{
    hodlvoice, hodlvoiceaccept, hodlvoicereject, hooks::invoice_payment_handler, PluginState,
    PLUGIN_NAME,
};
use log::debug;
use tokio::{self};
#[cfg(all(not(windows), not(target_env = "musl")))]
#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    std::env::set_var("CLN_PLUGIN_LOG", "trace");
    let state = PluginState::new();
    // let defaultconfig = Config::new();
    let confplugin;
    match Builder::new(tokio::io::stdin(), tokio::io::stdout())
        .rpcmethod(
            &(PLUGIN_NAME.to_string() + "-add"),
            "add hold-invoice",
            hodlvoice,
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
        .hook("invoice_payment", invoice_payment_handler)
        .dynamic()
        .configure()
        .await?
    {
        Some(plugin) => {
            // info!("read config");
            // match read_config(&plugin, state.clone()).await {
            //     Ok(()) => &(),
            //     Err(e) => return plugin.disable(format!("{}", e).as_str()).await,
            // };
            // info!("startup options");
            // match get_startup_options(&plugin, state.clone()) {
            //     Ok(()) => &(),
            //     Err(e) => return plugin.disable(format!("{}", e).as_str()).await,
            // };
            // match check_lightning_dir(&plugin, state.clone()).await {
            //     Ok(()) => &(),
            //     Err(e) => return plugin.disable(format!("{}", e).as_str()).await,
            // };
            confplugin = plugin;
        }
        None => return Err(anyhow!("Error configuring the plugin!")),
    };
    if let Ok(plugin) = confplugin.start(state).await {
        debug!("{:?}", plugin.configuration());
        // let mypubkey = get_info(&make_rpc_path(&plugin)).await?.id;
        // {
        //     plugin.state().config.lock().pubkey = Some(mypubkey);
        // }
        // let peersclone = plugin.clone();
        // tokio::spawn(async move {
        //     match tasks::refresh_listpeers(peersclone).await {
        //         Ok(()) => (),
        //         Err(e) => warn!("Error in refresh_listpeers thread: {:?}", e),
        //     };
        // });
        // let sling_dir = Path::new(&plugin.configuration().lightning_dir).join(PLUGIN_NAME);
        // read_excepts::<ShortChannelId>(
        //     plugin.state().excepts_chans.clone(),
        //     EXCEPTS_CHANS_FILE_NAME,
        //     &sling_dir,
        // )
        // .await?;
        // read_excepts::<PublicKey>(
        //     plugin.state().excepts_peers.clone(),
        //     EXCEPTS_PEERS_FILE_NAME,
        //     &sling_dir,
        // )
        // .await?;
        // let joblists_clone = plugin.clone();
        // refresh_joblists(joblists_clone).await?;
        // let channelsclone = plugin.clone();
        // tokio::spawn(async move {
        //     match tasks::refresh_graph(channelsclone).await {
        //         Ok(()) => (),
        //         Err(e) => warn!("Error in refresh_graph thread: {:?}", e),
        //     };
        // });
        // let aliasclone = plugin.clone();
        // tokio::spawn(async move {
        //     match tasks::refresh_aliasmap(aliasclone).await {
        //         Ok(()) => (),
        //         Err(e) => warn!("Error in refresh_aliasmap thread: {:?}", e),
        //     };
        // });
        // let liquidityclone = plugin.clone();
        // tokio::spawn(async move {
        //     match tasks::refresh_liquidity(liquidityclone).await {
        //         Ok(()) => (),
        //         Err(e) => warn!("Error in refresh_liquidity thread: {:?}", e),
        //     };
        // });
        // let tempbanclone = plugin.clone();
        // tokio::spawn(async move {
        //     match tasks::clear_tempbans(tempbanclone).await {
        //         Ok(()) => (),
        //         Err(e) => warn!("Error in clear_tempbans thread: {:?}", e),
        //     };
        // });
        // let clearstatsclone = plugin.clone();
        // tokio::spawn(async move {
        //     match tasks::clear_stats(clearstatsclone).await {
        //         Ok(()) => (),
        //         Err(e) => warn!("Error in clear_stats thread: {:?}", e),
        //     };
        // });
        plugin.join().await
    } else {
        Err(anyhow!("Error starting the plugin!"))
    }
}
