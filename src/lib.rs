pub mod token;
pub mod lexer;
pub mod error;
pub mod ast;
pub mod parser;
pub mod value;
pub mod env;
pub mod eval;

pub use error::{LexError, RunError};

use eval::Interpreter;


pub enum RunFailure {
    Frontend { line: usize, message: String },
    Runtime { error: RunError, output: Vec<String> },
}


pub fn run_source(
    source: &str,
    input: &str,
) -> Result<(Vec<String>, eval::Stats), RunFailure> {
    // 1. 词法分析；出错时还没执行，output 为空
    let tokens = lexer::tokenize(source).map_err(|e| RunFailure::Frontend {
        line: e.line,
        message: e.message,
    })?;

    // 2. 语法分析；同样还没执行
    let program = parser::parse(tokens).map_err(|e| RunFailure::Frontend {
        line: e.line,
        message: e.message,
    })?;

    // 3. 求值；错误已只含行号+消息，output 从 Interpreter 取出
    let interp = Interpreter::new(input);
    match interp.run(&program) {
        Ok(()) => {
            let output = interp.output.into_inner();
            let stats = interp.stats.into_inner();
            Ok((output, stats))
        }
        Err(e) => {
            // 求值出错：把出错前已 print 的内容一起取出
            let output = interp.output.into_inner();
            Err(RunFailure::Runtime { error: e, output })
        }
    }
}

#[cfg(target_arch = "wasm32")]
mod wasm_api {
    use super::*;
    use serde::Serialize;
    use wasm_bindgen::prelude::*;

    #[derive(Serialize)]
    struct StatsJson {
        comparisons: u64,
        assignments: u64,
        function_calls: u64,
        peak_element_count: u64,
    }

    #[derive(Serialize)]
    struct RunErrorJson {
        line: usize,
        message: String,
    }

    #[derive(Serialize)]
    struct RunResult {
        output: Vec<String>,
        stats: Option<StatsJson>,
        error: Option<RunErrorJson>,
    }

    #[wasm_bindgen(start)]
    pub fn init_panic_hook() {
        console_error_panic_hook::set_once();
    }

    #[wasm_bindgen]
    pub fn run(source: &str, input: &str) -> String {
        let result = match run_source(source, input) {
            Ok((output, stats)) => RunResult {
                output,
                stats: Some(StatsJson {
                    comparisons: stats.comparisons,
                    assignments: stats.assignments,
                    function_calls: stats.function_calls,
                    peak_element_count: stats.peak_element_count,
                }),
                error: None,
            },
            Err(RunFailure::Frontend { line, message }) => RunResult {
                output: vec![],   // 前端阶段错误：尚未执行，无输出
                stats: None,
                error: Some(RunErrorJson { line, message }),
            },
            Err(RunFailure::Runtime { error, output }) => RunResult {
                output,           // 出错前已 print 的内容
                stats: None,
                error: Some(RunErrorJson {
                    line: error.line,
                    message: error.message,
                }),
            },
        };
        serde_json::to_string(&result).unwrap_or_else(|_| {
            r#"{"output":[],"stats":null,"error":{"line":0,"message":"serialization failed"}}"#
                .to_string()
        })
    }
}

#[cfg(target_arch = "wasm32")]
pub use wasm_api::run;