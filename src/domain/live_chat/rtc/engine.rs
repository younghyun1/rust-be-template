//! Peer-connection construction for the in-process SFU.
//!
//! `webrtc` 0.20 owns one transport driver and UDP socket per peer connection.
//! The engine allocates those sockets from a bounded range and recreates the
//! connection-scoped media and interceptor configuration for each peer.

use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use tracing::info;
use webrtc::peer_connection::{
    MediaEngine, PeerConnection, PeerConnectionBuilder, PeerConnectionEventHandler,
    RTCConfigurationBuilder, RTCIceCandidateType, RTCIceServer, Registry, SettingEngine,
    register_default_interceptors,
};

use super::config::RtcConfig;

/// Builds peer connections and allocates their UDP ports.
pub struct RtcEngine {
    config: RtcConfig,
    next_port_slot: AtomicUsize,
}

impl RtcEngine {
    /// Validate that at least one configured media port can be bound.
    pub async fn new(config: RtcConfig) -> anyhow::Result<Self> {
        let port_end = configured_port(&config, config.max_participants - 1)?;
        let mut available = false;
        for port in config.udp_port_start..=port_end {
            if std::net::UdpSocket::bind((Ipv4Addr::UNSPECIFIED, port)).is_ok() {
                available = true;
                break;
            }
        }
        if !available {
            return Err(anyhow::anyhow!(
                "No RTC UDP port available in configured range {}-{port_end}",
                config.udp_port_start
            ));
        }

        info!(
            public_ip = %config.public_ip,
            udp_port_start = config.udp_port_start,
            udp_port_end = port_end,
            max_participants = config.max_participants,
            "RTC SFU engine initialized"
        );

        Ok(Self {
            config,
            next_port_slot: AtomicUsize::new(0),
        })
    }

    /// Maximum participants allowed per room call.
    pub fn max_participants(&self) -> usize {
        self.config.max_participants
    }

    /// Create a peer connection with its event handler and an available media port.
    pub async fn new_peer_connection(
        &self,
        handler: Arc<dyn PeerConnectionEventHandler>,
    ) -> anyhow::Result<Arc<dyn PeerConnection>> {
        let port_count = self.config.max_participants;
        let first_slot = self.next_port_slot.fetch_add(1, Ordering::Relaxed) % port_count;
        let mut last_error = None;

        for attempt in 0..port_count {
            let slot = (first_slot + attempt) % port_count;
            let port = configured_port(&self.config, slot)?;
            match self.build_peer_connection(port, handler.clone()).await {
                Ok(connection) => {
                    self.next_port_slot
                        .store((slot + 1) % port_count, Ordering::Relaxed);
                    return Ok(connection);
                }
                Err(error) => last_error = Some(error),
            }
        }

        let port_end = configured_port(&self.config, port_count - 1)?;
        let detail = last_error
            .map(|error| error.to_string())
            .unwrap_or_else(|| "no bind attempt completed".to_owned());
        Err(anyhow::anyhow!(
            "Failed to create RTC peer connection on UDP range {}-{port_end}: {detail}",
            self.config.udp_port_start
        ))
    }

    async fn build_peer_connection(
        &self,
        port: u16,
        handler: Arc<dyn PeerConnectionEventHandler>,
    ) -> anyhow::Result<Arc<dyn PeerConnection>> {
        let mut media_engine = MediaEngine::default();
        media_engine
            .register_default_codecs()
            .map_err(|error| anyhow::anyhow!("Failed to register RTC codecs: {error}"))?;
        let registry = register_default_interceptors(Registry::new(), &mut media_engine)
            .map_err(|error| anyhow::anyhow!("Failed to register RTC interceptors: {error}"))?;

        let mut setting_engine = SettingEngine::default();
        setting_engine.set_nat_1to1_ips(
            vec![self.config.public_ip.clone()],
            RTCIceCandidateType::Host,
        );

        let mut ice_servers = Vec::new();
        if let Some(turn) = &self.config.turn {
            ice_servers.push(RTCIceServer {
                urls: vec![turn.url.clone()],
                username: turn.username.clone(),
                credential: turn.credential.clone(),
            });
        }
        let configuration = RTCConfigurationBuilder::new()
            .with_ice_servers(ice_servers)
            .build();
        let bind_addr = SocketAddr::from((Ipv4Addr::UNSPECIFIED, port));

        let connection = PeerConnectionBuilder::<SocketAddr>::new()
            .with_configuration(configuration)
            .with_media_engine(media_engine)
            .with_setting_engine(setting_engine)
            .with_interceptor_registry(registry)
            .with_handler(handler)
            .with_udp_addrs(vec![bind_addr])
            .build()
            .await
            .map_err(|error| {
                anyhow::anyhow!("Failed to build RTC peer connection on UDP port {port}: {error}")
            })?;

        Ok(Arc::new(connection))
    }
}

fn configured_port(config: &RtcConfig, slot: usize) -> anyhow::Result<u16> {
    let offset = u16::try_from(slot)
        .map_err(|error| anyhow::anyhow!("RTC UDP port slot is out of range: {error}"))?;
    config
        .udp_port_start
        .checked_add(offset)
        .ok_or_else(|| anyhow::anyhow!("RTC UDP port range overflowed"))
}
