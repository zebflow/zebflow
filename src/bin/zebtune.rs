//! Zebtune binary: interactive automaton. Run and chat; each message is an objective.
//!
//! Set env (or .env) for real LLM:
//!   ZEBTUNE_LLM_PROVIDER=openai|anthropic
//!   OpenAI/OpenRouter: ZEBTUNE_OPENAI_API_KEY, ZEBTUNE_OPENAI_BASE_URL (default https://api.openai.com/v1), ZEBTUNE_OPENAI_MODEL (default gpt-4o-mini)
//!   Anthropic: ZEBTUNE_ANTHROPIC_API_KEY, ZEBTUNE_ANTHROPIC_MODEL (default claude-3-5-haiku-20241022)

use std::sync::Arc;

use zebflow::automaton::{
    AutomatonEngine, NoopAutomatonEngine, check_llm, log_llm_status, print_running_mechanism,
    run_interactive_with_llm,
};
use zebflow::llm;

fn main() -> std::io::Result<()> {
    let _ = dotenvy::dotenv();

    print_running_mechanism();

    let engine: Arc<dyn AutomatonEngine> = Arc::new(NoopAutomatonEngine);
    let llm_client = llm::client_from_env();

    if let Some(ref client) = llm_client {
        log_llm_status("LLM configured; running one-shot check...");
        let rt = tokio::runtime::Runtime::new().map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::Other, format!("runtime: {}", e))
        })?;
        match rt.block_on(check_llm(client.as_ref())) {
            Ok(()) => log_llm_status("LLM check OK."),
            Err(e) => {
                log_llm_status(&format!("LLM check failed: {}. First turn may fail.", e));
            }
        }
    } else {
        log_llm_status(
            "No LLM env set; using noop engine only. Set ZEBTUNE_OPENAI_API_KEY (or ZEBTUNE_ANTHROPIC_API_KEY) and optionally ZEBTUNE_LLM_PROVIDER.",
        );
    }

    run_interactive_with_llm(engine, llm_client)
}
