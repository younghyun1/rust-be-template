//! The shared webrtc-rs `API` for the SFU.
//!
//! Built once at startup and reused for every peer connection so that all ICE
//! traffic multiplexes onto a single UDP port and the server advertises its
//! public IP as a host candidate. Reusing one `API` across peers is the
//! standard webrtc-rs pattern.

use std::sync::Arc;

use tracing::info;
use webrtc::api::interceptor_registry::register_default_interceptors;
use webrtc::api::media_engine::MediaEngine;
use webrtc::api::setting_engine::SettingEngine;
use webrtc::api::{API, APIBuilder};
use webrtc::ice::udp_mux::{UDPMuxDefault, UDPMuxParams};
use webrtc::ice::udp_network::UDPNetwork;
use webrtc::ice_transport::ice_candidate_type::RTCIceCandidateType;
use webrtc::ice_transport::ice_server::RTCIceServer;
use webrtc::interceptor::registry::Registry;
use webrtc::peer_connection::RTCPeerConnection;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::rtp_transceiver::RTCPFeedback;
use webrtc::rtp_transceiver::rtp_codec::{
    RTCRtpCodecCapability, RTCRtpCodecParameters, RTPCodecType,
};

use super::config::RtcConfig;

/// AV1 MIME type. Not a built-in constant in webrtc-rs 0.17, so registered
/// manually; the SFU forwards its RTP opaquely (no transcoding).
const MIME_TYPE_AV1: &str = "video/AV1";
/// Dynamic payload type used when registering AV1. Codec matching during
/// negotiation is by MIME type, so the exact value only needs to be free.
const AV1_PAYLOAD_TYPE: u8 = 45;

/// Holds the shared SFU `API` and a clone of the config it was built from.
pub struct RtcEngine {
    api: API,
    config: RtcConfig,
}

impl RtcEngine {
    /// Build the SFU engine: register codecs, bind the UDP mux socket, and
    /// configure the public-IP host candidate. Returns an error if the UDP
    /// port cannot be bound or the media engine cannot be configured.
    pub async fn new(config: RtcConfig) -> anyhow::Result<Self> {
        let mut media_engine = MediaEngine::default();
        media_engine
            .register_default_codecs()
            .map_err(|e| anyhow::anyhow!("Failed to register default codecs: {e}"))?;
        register_av1_best_effort(&mut media_engine);

        // NACK, RTCP sender/receiver reports, and TWCC. Without these the SDP
        // advertises feedback the SFU never honors: publishers get no receiver
        // reports (blind bandwidth estimation) and lost packets are never
        // retransmitted, so video degrades unrecoverably on first loss.
        let registry = register_default_interceptors(Registry::new(), &mut media_engine)
            .map_err(|e| anyhow::anyhow!("Failed to register default interceptors: {e}"))?;

        let mut setting_engine = SettingEngine::default();
        let udp_socket =
            tokio::net::UdpSocket::bind((std::net::Ipv4Addr::UNSPECIFIED, config.udp_mux_port))
                .await
                .map_err(|e| {
                    anyhow::anyhow!(
                        "Failed to bind RTC UDP mux port {}: {e}",
                        config.udp_mux_port
                    )
                })?;
        let udp_mux = UDPMuxDefault::new(UDPMuxParams::new(udp_socket));
        setting_engine.set_udp_network(UDPNetwork::Muxed(udp_mux));
        setting_engine.set_nat_1to1_ips(vec![config.public_ip.clone()], RTCIceCandidateType::Host);

        let api = APIBuilder::new()
            .with_media_engine(media_engine)
            .with_setting_engine(setting_engine)
            .with_interceptor_registry(registry)
            .build();

        info!(
            public_ip = %config.public_ip,
            udp_mux_port = config.udp_mux_port,
            max_participants = config.max_participants,
            "RTC SFU engine initialized"
        );

        Ok(Self { api, config })
    }

    /// Maximum participants allowed per room call.
    pub fn max_participants(&self) -> usize {
        self.config.max_participants
    }

    /// Create a new peer connection on the shared API.
    pub async fn new_peer_connection(&self) -> anyhow::Result<Arc<RTCPeerConnection>> {
        let mut ice_servers = Vec::new();
        if let Some(turn) = &self.config.turn {
            ice_servers.push(RTCIceServer {
                urls: vec![turn.url.clone()],
                username: turn.username.clone(),
                credential: turn.credential.clone(),
            });
        }

        let configuration = RTCConfiguration {
            ice_servers,
            ..Default::default()
        };

        let pc = self
            .api
            .new_peer_connection(configuration)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create peer connection: {e}"))?;
        Ok(Arc::new(pc))
    }
}

/// Register AV1 as an additional video codec. Best-effort: if registration
/// fails the SFU still works with the default codecs (VP8/VP9/H264/Opus), so a
/// failure is logged and otherwise ignored. The SFU forwards AV1 RTP opaquely.
fn register_av1_best_effort(media_engine: &mut MediaEngine) {
    let video_feedback = vec![
        RTCPFeedback {
            typ: "goog-remb".to_owned(),
            parameter: String::new(),
        },
        RTCPFeedback {
            typ: "ccm".to_owned(),
            parameter: "fir".to_owned(),
        },
        RTCPFeedback {
            typ: "nack".to_owned(),
            parameter: String::new(),
        },
        RTCPFeedback {
            typ: "nack".to_owned(),
            parameter: "pli".to_owned(),
        },
        RTCPFeedback {
            typ: "transport-cc".to_owned(),
            parameter: String::new(),
        },
    ];

    let av1 = RTCRtpCodecParameters {
        capability: RTCRtpCodecCapability {
            mime_type: MIME_TYPE_AV1.to_owned(),
            clock_rate: 90_000,
            channels: 0,
            sdp_fmtp_line: String::new(),
            rtcp_feedback: video_feedback,
        },
        payload_type: AV1_PAYLOAD_TYPE,
        stats_id: String::new(),
    };

    match media_engine.register_codec(av1, RTPCodecType::Video) {
        Ok(_) => info!(
            mime_type = MIME_TYPE_AV1,
            "Registered AV1 video codec for SFU"
        ),
        Err(e) => tracing::warn!(error = %e, "AV1 codec registration failed; using default codecs"),
    }
}
