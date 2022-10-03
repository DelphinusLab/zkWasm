use crate::runtime::{WasmInterpreter, WasmRuntime};
use specs::types::Value;
use std::{collections::HashMap, fs};
use wasmi::{ImportsBuilder, NopExternals};
use wast::{lexer::Lexer, parser::ParseBuffer, Error, Wast, WastArg};

fn run_spec_test(file_name: &str) -> Result<(), Error> {
    let path = format!("src/test/spec/{}.wast", file_name);
    let file = fs::read_to_string(&path).unwrap();

    let mut lexer = Lexer::new(&file);
    lexer.allow_confusing_unicode(true);
    let parse_buffer = ParseBuffer::new_with_lexer(lexer)?;

    let wast: Wast = wast::parser::parse(&parse_buffer)?;
    let imports = ImportsBuilder::default();
    let mut externals = NopExternals;

    let compiler = WasmInterpreter::new();
    let mut compile_outcome = None;

    for directive in wast.directives {
        match directive {
            wast::WastDirective::Wat(wat) => match wat {
                wast::QuoteWat::Wat(wat) => match wat {
                    wast::Wat::Module(module) => {
                        let compiled = compiler
                            .compile_from_wast(module, &imports, HashMap::default())
                            .unwrap();
                        compile_outcome = Some(compiled);
                    }
                    wast::Wat::Component(_) => todo!(),
                },
                wast::QuoteWat::QuoteModule(_, _) => todo!(),
                wast::QuoteWat::QuoteComponent(_, _) => todo!(),
            },
            wast::WastDirective::AssertMalformed { .. } => todo!(),
            wast::WastDirective::AssertInvalid { .. } => todo!(),
            wast::WastDirective::Register { .. } => todo!(),
            wast::WastDirective::Invoke(..) => todo!(),
            wast::WastDirective::AssertTrap { .. } => todo!(),
            wast::WastDirective::AssertReturn {
                span: _span,
                exec,
                results: _results,
            } => {
                let from_wastarg = |arg: &WastArg| match arg {
                    WastArg::Core(core) => match core {
                        wast::core::WastArgCore::I32(v) => Value::I32(*v),
                        wast::core::WastArgCore::I64(v) => Value::I64(*v),
                        wast::core::WastArgCore::F32(_) => todo!(),
                        wast::core::WastArgCore::F64(_) => todo!(),
                        wast::core::WastArgCore::V128(_) => todo!(),
                        wast::core::WastArgCore::RefNull(_) => todo!(),
                        wast::core::WastArgCore::RefExtern(_) => todo!(),
                    },
                    WastArg::Component(_) => todo!(),
                };

                let _actual_results = match exec {
                    wast::WastExecute::Invoke(invoke) => compiler.run(
                        &mut externals,
                        &compile_outcome.unwrap(),
                        invoke.name,
                        invoke.args.iter().map(|arg| from_wastarg(arg)).collect(),
                    ),
                    wast::WastExecute::Wat(_) => todo!(),
                    wast::WastExecute::Get { .. } => todo!(),
                }
                .unwrap();

                todo!()
            }
            wast::WastDirective::AssertExhaustion { .. } => todo!(),
            wast::WastDirective::AssertUnlinkable { .. } => todo!(),
            wast::WastDirective::AssertException { .. } => todo!(),
        }
    }
    Ok(())
}

/*
mod tests {
    use super::run_spec_test;

    #[test]
    fn test_spec_i32() {
        run_spec_test("i32").unwrap()
    }
}
*/
