mod node_id;
mod dump;
mod persistence;
mod web;
mod config;
mod graph;
mod mac;
mod meshinfo;

use tokio::sync::{mpsc, RwLock, Mutex};
use tokio::{task, fs, time};
use std::sync::Arc;
use crate::config::SiteConfig;
use sd_notify::NotifyState;
use std::collections::HashMap;
use crate::graph::UpdatePolicy;
use tokio::stream::StreamExt;
use std::net::SocketAddr;
use clap::clap_app;
use crate::persistence::{PersistentState, LinkInfo};
use std::path::PathBuf;

pub struct MainState {
    graphs: HashMap<(String, String), Arc<SiteState>>,
    listen_addr: SocketAddr
}

pub struct SiteState {
    graph: RwLock<graph::Graph>,
    persistent: Arc<Mutex<persistence::PersistentState>>,
    persistent_saver: mpsc::Sender<()>,
    config: SiteConfig
}

fn args<'a, 'b>() -> clap::App<'a, 'b> {
    clap_app!(gluon_update_manager =>
        (author: "Stephan Henrichs <kilobyte+gluon-update-mgr@kilobyte22.de>")
        (@arg config: -c --config +takes_value +required "Config File")
    )
}

async fn generate_graph(
    config: &SiteConfig,
    persistent: &mut PersistentState
) -> Result<graph::Graph, failure::Error> {
    let meshinfo = reqwest::get(&config.meshinfo)
        .await?
        .json()
        .await?;

    let graph = graph::Graph::build(&meshinfo, config, persistent);

    let now = chrono::Utc::now();
    for (_, node) in &graph.nodes {
        if !persistent.link_history.contains_key(&node.node.node_id) {
            if let Some(uplink_key) = &node.uplink {
                if let Some(uplink_node) = graph.nodes.get(*uplink_key) {
                    persistent.link_history.insert(node.node.node_id.clone(), LinkInfo {
                        uplink: uplink_node.node.node_id.clone(),
                        since: now
                    });
                }
            }
        }
    }

    Ok(graph)
}

async fn configurator_task(
    site: Arc<SiteState>,
    mut updater: mpsc::Sender<()>
) -> Result<(), failure::Error> {
    let mut persistent_saver = site.persistent_saver.clone();
    loop {
        time::delay_for(time::Duration::from_secs(site.config.refresh_interval)).await;

        log::debug!("Refreshing node graph for site {}/{}", site.config.name, site.config.branch);
        match generate_graph(&site.config, &mut *site.persistent.lock().await).await {
            Ok(new_graph) => {
                persistent_saver.send(()).await?;

                let mut graph = site.graph.write().await;
                *graph = new_graph;
                updater.send(()).await?;
            },
            Err(e) => {
                log::error!(
                    "Failed to refresh node graph for site {}/{}: {}",
                    site.config.name,
                    site.config.branch,
                    e
                )
            }
        }
    }
}

async fn persitent_saver(
    site: Arc<Mutex<PersistentState>>,
    file: PathBuf,
    mut rx: mpsc::Receiver<()>,
) -> Result<(), failure::Error> {
    while let Some(()) = rx.next().await {
        log::debug!("Writing persistent state to {:?}", file);
        fs::write(&file, &serde_json::to_string_pretty(&*site.lock().await)?).await?;
    }
    Ok(())
}

#[actix_web::main]
async fn main() -> Result<(), failure::Error> {

    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    let matches = args().get_matches();

    let conf_file = matches.value_of("config").unwrap();

    let config: config::Config = toml::from_str(&fs::read_to_string(conf_file).await?)?;

    let (mut state_tx, state_rx) = mpsc::channel(8);

    let mut site_map = HashMap::new();
    for site in config.sites {
        log::info!("Preparing site {}/{}...", site.name, site.branch);

        let persistent = Arc::new(Mutex::new(if site.state_file.exists() {
            serde_json::from_str(&fs::read_to_string(&site.state_file).await?)?
        } else {
            Default::default()
        }));

        let (mut pers_tx, pers_rx) = mpsc::channel(8);

        let state = Arc::new(SiteState {
            graph: RwLock::new(generate_graph(&site, &mut *persistent.lock().await).await?),
            persistent: persistent.clone(),
            persistent_saver: pers_tx.clone(),
            config: site.clone()
        });
        pers_tx.send(()).await?;

        site_map.insert((site.name.clone(), site.branch.clone()), state.clone());

        task::spawn(persitent_saver(persistent.clone(), site.state_file.clone(), pers_rx));

        log::info!("Spawning site {}/{} background task", site.name, site.branch);
        task::spawn(configurator_task(state, state_tx.clone()));
    }

    state_tx.send(()).await?;

    let state = Arc::new(MainState {
        graphs: site_map,
        listen_addr: config.listen.clone()
    });

    task::spawn(push_state_to_systemd_task(state.clone(), state_rx));

    sd_notify::notify(false, &[NotifyState::Ready])?;

    web::main(state).await?;

    Ok(())
}

async fn push_state_to_systemd_task(state: Arc<MainState>, mut recv: mpsc::Receiver<()>) -> Result<(), failure::Error> {
    while let Some(()) = recv.next().await {
        let mut res = vec![];
        for ((site_name, branch), site) in &state.graphs {
            let graph = site.graph.read().await;
            let migrated = graph.update_policy
                .values()
                .filter(|p| **p == UpdatePolicy::Finished)
                .count();
            let cleared = graph.update_policy
                .values()
                .filter(|p| **p == UpdatePolicy::Ready)
                .count();
            let pending = graph.update_policy
                .values()
                .filter(|p| **p == UpdatePolicy::Pending)
                .count();
            let total = graph.nodes.len();
            res.push(format!(
                "{}/{}: {}/{}/{}/{}",
                site_name, branch,
                migrated, cleared, pending, total
            ))
        }
        let status = res.join(", ") + " migrated/cleared/blocked/total";
        log::debug!("Status update: {}", status);
        sd_notify::notify(false, &[NotifyState::Status(status)])?;
    }
    Ok(())
}