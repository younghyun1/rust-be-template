use std::sync::Arc;
use std::sync::atomic::Ordering;

use tokio::process::Command;
use tracing::{Instrument, error};

use crate::init::state::ServerState;

pub async fn update_fastfetch_cache(state: Arc<ServerState>) {
    let span = tracing::info_span!("update_fastfetch_cache");
    async move {
        // Run the 'fastfetch' command asynchronously
        match Command::new("fastfetch").output().await {
            Ok(output) => {
                // Convert stdout to a String, assuming UTF-8/ANSI output
                let ansi_output = String::from_utf8_lossy(&output.stdout).to_string();

                // Convert ANSI to HTML
                match ansi_to_html::convert(&ansi_output) {
                    Ok(html_output) => {
                        // Wrap in <pre> with CSS to ensure proper monospace and color rendering,
                        // and provide fallback colors for -- variables in case they are not supported
                        let css_prefix = r#"
                            <style>
                                pre.ansi {
                                    font-family: SFMono-Regular, Menlo, Monaco, 'Liberation Mono', Consolas, monospace;
                                    background: #222;
                                    color: #ddd;
                                    padding: 1em;
                                    border-radius: 8px;
                                    overflow-x: auto;
                                    margin: 0;
                                    font-size: 0.98em;
                                }
                                :root {
                                    --black:  #000;
                                    --red:    #a00;
                                    --green:  #0a0;
                                    --yellow: #a60;
                                    --blue:   #00a;
                                    --magenta:#a0a;
                                    --cyan:   #0aa;
                                    --white:  #aaa;
                                    --bright-black:   #555;
                                    --bright-red:     #f55;
                                    --bright-green:   #5f5;
                                    --bright-yellow:  #ff5;
                                    --bright-blue:    #55f;
                                    --bright-magenta: #f5f;
                                    --bright-cyan:    #5ff;
                                    --bright-white:   #fff;
                                }
                            </style>
                        "#;
                        let html_wrapped = format!(
                            "{css}<pre class='ansi'>{}</pre>",
                            html_output,
                            css = css_prefix
                        );

                        let mut cache = state.fastfetch_cache.write().await;
                        *cache = Some(html_wrapped.clone());
                        state.fastfetch_cache_exists.store(true, Ordering::Relaxed);
                    }
                    Err(e) => {
                        error!(error = ?e, "Failed to convert fastfetch output to HTML");
                    }
                }
            }
            Err(e) => {
                error!(error = ?e, "Failed to execute fastfetch command");
            }
        }
    }
    .instrument(span)
    .await;
}
