use wasmi::{
    Error, Externals, FuncInstance, ModuleImportResolver, RuntimeArgs, RuntimeValue, Signature,
    Trap, ValueType,
};

struct TestHost {}

impl TestHost {
    fn new() -> TestHost {
        TestHost {}
    }
}

impl Externals for TestHost {
    fn invoke_index(
        &mut self,
        index: usize,
        _args: RuntimeArgs,
    ) -> Result<Option<RuntimeValue>, Trap> {
        match index {
            0 => Ok(Some(RuntimeValue::I32(0))),
            1 => Ok(Some(RuntimeValue::I32(0))),
            2 => Ok(Some(RuntimeValue::I32(0))),
            3 => Ok(Some(RuntimeValue::I32(0))),
            4 => Ok(Some(RuntimeValue::I32(0))),
            5 => Ok(Some(RuntimeValue::I32(0))),
            _ => panic!("env doesn't provide function at index {}", index),
        }
    }
}

impl TestHost {
    fn check_signature(&self, index: usize, signature: &Signature) -> bool {
        let (params, ret_ty): (&[ValueType], Option<ValueType>) = match index {
            0 => (&[ValueType::I32], Some(ValueType::I32)),
            1 => (
                &[ValueType::I32, ValueType::I32, ValueType::I32],
                Some(ValueType::I32),
            ),
            2 => (&[ValueType::I32], Some(ValueType::I32)),
            3 => (
                &[ValueType::I32, ValueType::I32, ValueType::I32],
                Some(ValueType::I32),
            ),
            4 => (&[ValueType::I32], Some(ValueType::I32)),
            5 => (&[ValueType::I32], Some(ValueType::I32)),
            _ => return false,
        };

        signature.params() == params && signature.return_type() == ret_ty
    }
}

impl ModuleImportResolver for TestHost {
    fn resolve_func(
        &self,
        field_name: &str,
        signature: &wasmi::Signature,
    ) -> Result<wasmi::FuncRef, wasmi::Error> {
        let index = match field_name {
            "Sigma1" => 0,
            "Ch" => 1,
            "Sigma0" => 2,
            "Maj" => 3,
            "sigma1" => 4,
            "sigma0" => 5,
            _ => {
                return Err(Error::Instantiation(format!(
                    "Export {} not found",
                    field_name
                )));
            }
        };

        if !self.check_signature(index, signature) {
            return Err(Error::Instantiation(format!(
                "Export `{}` doesnt match expected type {:?}",
                field_name, signature
            )));
        }

        Ok(FuncInstance::alloc_host(signature.clone(), index))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        runtime::{WasmInterpreter, WasmRuntime},
        test::run_test_circuit,
    };

    use halo2_proofs::pairing::bn256::Fr as Fp;
    use specs::types::Value;
    use std::{fs::File, io::Read, path::PathBuf};
    use wasmi::ImportsBuilder;

    #[test]
    fn test_sha256() {
        let mut wasm = vec![];

        let path = PathBuf::from("wasm/sha256_extern.wasm");
        let mut f = File::open(path).unwrap();
        f.read_to_end(&mut wasm).unwrap();

        let compiler = WasmInterpreter::new();
        let mut env = TestHost::new();
        let imports = ImportsBuilder::new().with_resolver("env", &env);
        let compiled_module = compiler.compile(&wasm, &imports).unwrap();
        let execution_log = compiler
            .run(
                &mut env,
                &compiled_module,
                "Hash_Calculate",
                vec![Value::I32(1024), Value::I32(256)],
            )
            .unwrap();
        run_test_circuit::<Fp>(compiled_module.tables, execution_log.tables).unwrap()
    }
}
