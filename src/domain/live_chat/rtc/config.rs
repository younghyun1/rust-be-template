//! Configuration for the in-process WebRTC SFU, loaded from the environment.

use nutype::nutype;
use tracing::warn;

/// Default upper bound on simultaneous participants in a single room call.
const DEFAULT_RTC_MAX_PARTICIPANTS: usize = 16;

/// Validated maximum number of participants allowed in one room call. Used to
/// range-check the `RTC_MAX_PARTICIPANTS` env value at parse time; the validated
/// inner value is stored as a plain `usize` on [`RtcConfig`].
#[nutype(
    validate(greater_or_equal = 1, less_or_equal = 64),
    derive(Debug, Clone, Copy, PartialEq, Eq)
)]
pub struct MaxParticipants(usize);

/// Optional TURN relay used as a fallback for symmetric-NAT clients. Not needed
/// when the SFU has a reachable public IP (the common deployment).
#[derive(Debug, Clone)]
pub struct TurnConfig {
    pub url: String,
    pub username: String,
    pub credential: String,
}

/// Runtime configuration for the SFU.
#[derive(Debug, Clone)]
pub struct RtcConfig {
    /// When false the SFU is not built and RTC join requests are rejected.
    pub enabled: bool,
    /// Public IP advertised as an ICE host candidate (`set_nat_1to1_ips`).
    pub public_ip: String,
    /// First UDP port in the per-peer ICE/media port range.
    pub udp_port_start: u16,
    /// Maximum participants per room call (validated to `1..=64`).
    pub max_participants: usize,
    /// Optional TURN relay fallback.
    pub turn: Option<TurnConfig>,
}

impl RtcConfig {
    /// Load SFU configuration from environment variables.
    ///
    /// `RTC_ENABLE` gates the whole feature (default off). When enabled,
    /// `RTC_PUBLIC_IP` and `RTC_UDP_PORT_START` are required; missing/invalid
    /// values disable the SFU rather than aborting startup, so the rest of the
    /// server still comes up.
    pub fn from_env() -> Self {
        let enabled = std::env::var("RTC_ENABLE")
            .ok()
            .map(|value| {
                matches!(
                    value.trim().to_ascii_lowercase().as_str(),
                    "1" | "true" | "yes" | "on"
                )
            })
            .unwrap_or(false);

        let max_participants = std::env::var("RTC_MAX_PARTICIPANTS")
            .ok()
            .and_then(|value| value.trim().parse::<usize>().ok())
            .and_then(|value| MaxParticipants::try_new(value).ok())
            .map(MaxParticipants::into_inner)
            .unwrap_or(DEFAULT_RTC_MAX_PARTICIPANTS);

        let turn = match (
            std::env::var("RTC_TURN_URL").ok(),
            std::env::var("RTC_TURN_USER").ok(),
            std::env::var("RTC_TURN_PASS").ok(),
        ) {
            (Some(url), Some(username), Some(credential)) if !url.trim().is_empty() => {
                Some(TurnConfig {
                    url,
                    username,
                    credential,
                })
            }
            _ => None,
        };

        if !enabled {
            return Self::disabled(max_participants, turn);
        }

        let public_ip = match std::env::var("RTC_PUBLIC_IP") {
            Ok(value) if !value.trim().is_empty() => value,
            _ => {
                warn!("RTC_ENABLE set but RTC_PUBLIC_IP missing; disabling SFU");
                return Self::disabled(max_participants, turn);
            }
        };

        let udp_port_start = match std::env::var("RTC_UDP_PORT_START")
            .ok()
            .and_then(|value| value.trim().parse::<u16>().ok())
        {
            Some(port) if port != 0 => port,
            _ => {
                warn!("RTC_ENABLE set but RTC_UDP_PORT_START missing/invalid; disabling SFU");
                return Self::disabled(max_participants, turn);
            }
        };
        if !udp_port_range_fits(udp_port_start, max_participants) {
            warn!(
                udp_port_start,
                max_participants, "RTC UDP port range exceeds port 65535; disabling SFU"
            );
            return Self::disabled(max_participants, turn);
        }

        Self {
            enabled: true,
            public_ip,
            udp_port_start,
            max_participants,
            turn,
        }
    }

    fn disabled(max_participants: usize, turn: Option<TurnConfig>) -> Self {
        Self {
            enabled: false,
            public_ip: String::new(),
            udp_port_start: 0,
            max_participants,
            turn,
        }
    }
}

/// Whether a contiguous range starting at `start` has `port_count` valid ports.
fn udp_port_range_fits(start: u16, port_count: usize) -> bool {
    let end_exclusive = usize::from(start).saturating_add(port_count);
    end_exclusive <= usize::from(u16::MAX) + 1
}

#[cfg(test)]
mod tests {
    use super::udp_port_range_fits;

    #[test]
    fn udp_port_range_accepts_last_valid_port() {
        assert!(udp_port_range_fits(u16::MAX, 1));
        assert!(udp_port_range_fits(u16::MAX - 63, 64));
    }

    #[test]
    fn udp_port_range_rejects_overflow() {
        assert!(!udp_port_range_fits(u16::MAX, 2));
        assert!(!udp_port_range_fits(u16::MAX - 62, 64));
    }
}
