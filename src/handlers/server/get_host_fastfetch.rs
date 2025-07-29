use std::sync::{Arc, atomic::Ordering};

use axum::{extract::State, response::IntoResponse};
use tokio::process::Command;

use crate::{
    dto::responses::response_data::http_resp,
    errors::code_error::{CodeError, HandlerResponse, code_err},
    init::state::ServerState,
    util::time::now::tokio_now,
};

pub async fn get_host_fastfetch(
    State(state): State<Arc<ServerState>>,
) -> HandlerResponse<impl IntoResponse> {
    let start = tokio_now();

    if state.fastfetch_cache_exists.load(Ordering::Relaxed) {
        let cache = state.fastfetch_cache.read().await;
        return Ok(http_resp(
            unsafe { cache.clone().unwrap_unchecked() },
            (),
            start,
        ));
    }

    // Run the 'fastfetch' command asynchronously
    let output = Command::new("fastfetch")
        .output()
        .await
        .map_err(|e| code_err(CodeError::COULD_NOT_RUN_FASTFETCH, e))?;

    // Convert stdout to a String, assuming UTF-8/ANSI output
    let ansi_output = String::from_utf8_lossy(&output.stdout).to_string();

    // Convert ANSI to HTML
    let html_output = ansi_to_html::convert(&ansi_output)
        .map_err(|e| code_err(CodeError::ANSI_TO_HTML_FAILED, e))?;

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

    {
        let mut cache = state.fastfetch_cache.write().await;
        *cache = Some(html_wrapped.clone());
        state.fastfetch_cache_exists.store(true, Ordering::Relaxed);
    }

    Ok(http_resp(html_wrapped, (), start))
}
