use wasm_bindgen_test::*;

use wasmer::*;

#[wasm_bindgen_test]
fn global_new() {
    let mut store = Store::default();
    let global = Global::new(&mut store, Value::I32(10));
    assert_eq!(
        global.ty(&store),
        GlobalType {
            ty: Type::I32,
            mutability: Mutability::Const
        }
    );

    let global_mut = Global::new_mut(&mut store, Value::I32(10));
    assert_eq!(
        global_mut.ty(&store),
        GlobalType {
            ty: Type::I32,
            mutability: Mutability::Var
        }
    );
}

#[wasm_bindgen_test]
fn global_get() {
    let mut store = Store::default();

    let global_i32 = Global::new(&mut store, Value::I32(10));
    assert_eq!(global_i32.get(&mut store), Value::I32(10));

    let global_f32 = Global::new(&mut store, Value::F32(10.0));
    assert_eq!(global_f32.get(&mut store), Value::F32(10.0));
}

#[wasm_bindgen_test]
fn global_set() {
    let mut store = Store::default();
    let global_i32 = Global::new(&mut store, Value::I32(10));
    // Set on a constant should error
    assert!(global_i32.set(&mut store, Value::I32(20)).is_err());

    let global_i32_mut = Global::new_mut(&mut store, Value::I32(10));
    // Set on different type should error
    assert!(global_i32_mut.set(&mut store, Value::I64(20)).is_err());

    // Set on same type should succeed
    global_i32_mut
        .set(&mut store, Value::I32(20))
        .map_err(|e| format!("{e:?}"))
        .unwrap();
    assert_eq!(global_i32_mut.get(&mut store), Value::I32(20));
}

#[wasm_bindgen_test]
fn table_new() {
    let mut store = Store::default();
    let table_type = TableType {
        ty: Type::FuncRef,
        minimum: 0,
        maximum: None,
    };
    let f = Function::new_typed(&mut store, || {});
    let table = Table::new(&mut store, table_type, Value::FuncRef(Some(f)))
        .map_err(|e| format!("{e:?}"))
        .unwrap();
    assert_eq!(table.ty(&store), table_type);

    // Anyrefs not yet supported
    // let table_type = TableType {
    //     ty: Type::ExternRef,
    //     minimum: 0,
    //     maximum: None,
    // };
    // let table = Table::new(&store, table_type, Value::ExternRef(ExternRef::Null)).map_err(|e| format!("{e:?}"))?;
    // assert_eq!(*table.ty(), table_type);
}

#[wasm_bindgen_test]
fn table_get() {
    // Tables are not yet fully supported in Wasm
    // This test was marked as #[ignore] on -sys, which is why it is commented out.

    //    let mut store = Store::default();
    //    let table_type = TableType {
    //        ty: Type::FuncRef,
    //        minimum: 0,
    //        maximum: Some(1),
    //    };
    //    let f = Function::new_typed(&mut store, |num: i32| num + 1);
    //    let table = Table::new(&mut store, table_type, Value::FuncRef(Some(f)))
    //        .map_err(|e| format!("{e:?}"))?;
    //    assert_eq!(table.ty(&mut store), table_type);
    //    let _elem = table.get(&mut store, 0).unwrap();
    //    assert_eq!(elem.funcref().unwrap(), f);
}

#[wasm_bindgen_test]
fn table_set() {
    // Table set not yet tested
}

#[wasm_bindgen_test]
fn table_grow() {
    // Tables are not yet fully supported in Wasm
}

#[wasm_bindgen_test]
fn table_copy() {
    // TODO: table copy test not yet implemented
}

#[wasm_bindgen_test]
fn memory_new() {
    let mut store = Store::default();
    let memory_type = MemoryType {
        shared: false,
        minimum: Pages(0),
        maximum: Some(Pages(10)),
    };
    let memory = Memory::new(&mut store, memory_type)
        .map_err(|e| format!("{e:?}"))
        .unwrap();
    assert_eq!(memory.view(&store).size(), Pages(0));
    assert_eq!(memory.ty(&store), memory_type);
}

#[wasm_bindgen_test]
fn memory_grow() {
    let mut store = Store::default();
    let desc = MemoryType::new(Pages(10), Some(Pages(16)), false);
    let memory = Memory::new(&mut store, desc)
        .map_err(|e| format!("{e:?}"))
        .unwrap();
    assert_eq!(memory.view(&store).size(), Pages(10));

    let result = memory.grow(&mut store, Pages(2)).unwrap();
    assert_eq!(result, Pages(10));
    assert_eq!(memory.view(&store).size(), Pages(12));

    let result = memory.grow(&mut store, Pages(10));
    assert_eq!(
        result,
        Err(MemoryError::CouldNotGrow {
            current: 12.into(),
            attempted_delta: 10.into()
        })
    );
}

#[wasm_bindgen_test]
fn function_new() {
    let mut store = Store::default();
    let function = Function::new_typed(&mut store, || {});
    assert_eq!(function.ty(&store), FunctionType::new(vec![], vec![]));
    let function = Function::new_typed(&mut store, |_a: i32| {});
    assert_eq!(
        function.ty(&store),
        FunctionType::new(vec![Type::I32], vec![])
    );
    let function = Function::new_typed(&mut store, |_a: i32, _b: i64, _c: f32, _d: f64| {});
    assert_eq!(
        function.ty(&store),
        FunctionType::new(vec![Type::I32, Type::I64, Type::F32, Type::F64], vec![])
    );
    let function = Function::new_typed(&mut store, || -> i32 { 1 });
    assert_eq!(
        function.ty(&store),
        FunctionType::new(vec![], vec![Type::I32])
    );
    let function = Function::new_typed(&mut store, || -> (i32, i64, f32, f64) { (1, 2, 3.0, 4.0) });
    assert_eq!(
        function.ty(&store),
        FunctionType::new(vec![], vec![Type::I32, Type::I64, Type::F32, Type::F64])
    );
}

#[wasm_bindgen_test]
fn function_new_env() {
    let mut store = Store::default();
    #[derive(Clone)]
    struct MyEnv {}

    let my_env = MyEnv {};
    let env = FunctionEnv::new(&mut store, my_env);
    let function = Function::new_typed_with_env(&mut store, &env, |_env: FunctionEnvMut<MyEnv>| {});
    assert_eq!(function.ty(&store), FunctionType::new(vec![], vec![]));
    let function =
        Function::new_typed_with_env(&mut store, &env, |_env: FunctionEnvMut<MyEnv>, _a: i32| {});
    assert_eq!(
        function.ty(&store),
        FunctionType::new(vec![Type::I32], vec![])
    );
    let function = Function::new_typed_with_env(
        &mut store,
        &env,
        |_env: FunctionEnvMut<MyEnv>, _a: i32, _b: i64, _c: f32, _d: f64| {},
    );
    assert_eq!(
        function.ty(&store),
        FunctionType::new(vec![Type::I32, Type::I64, Type::F32, Type::F64], vec![])
    );
    let function =
        Function::new_typed_with_env(&mut store, &env, |_env: FunctionEnvMut<MyEnv>| -> i32 { 1 });
    assert_eq!(
        function.ty(&store),
        FunctionType::new(vec![], vec![Type::I32])
    );
    let function = Function::new_typed_with_env(
        &mut store,
        &env,
        |_env: FunctionEnvMut<MyEnv>| -> (i32, i64, f32, f64) { (1, 2, 3.0, 4.0) },
    );
    assert_eq!(
        function.ty(&store),
        FunctionType::new(vec![], vec![Type::I32, Type::I64, Type::F32, Type::F64])
    );
}

#[wasm_bindgen_test]
fn function_new_dynamic() {
    let mut store = Store::default();

    // Using &FunctionType signature
    let function_type = FunctionType::new(vec![], vec![]);
    let function = Function::new(
        &mut store,
        &function_type,
        |_values: &[Value]| unimplemented!(),
    );
    assert_eq!(function.ty(&store), function_type);
    let function_type = FunctionType::new(vec![Type::I32], vec![]);
    let function = Function::new(
        &mut store,
        &function_type,
        |_values: &[Value]| unimplemented!(),
    );
    assert_eq!(function.ty(&store), function_type);
    let function_type = FunctionType::new(vec![Type::I32, Type::I64, Type::F32, Type::F64], vec![]);
    let function = Function::new(
        &mut store,
        &function_type,
        |_values: &[Value]| unimplemented!(),
    );
    assert_eq!(function.ty(&store), function_type);
    let function_type = FunctionType::new(vec![], vec![Type::I32]);
    let function = Function::new(
        &mut store,
        &function_type,
        |_values: &[Value]| unimplemented!(),
    );
    assert_eq!(function.ty(&store), function_type);
    let function_type = FunctionType::new(vec![], vec![Type::I32, Type::I64, Type::F32, Type::F64]);
    let function = Function::new(
        &mut store,
        &function_type,
        |_values: &[Value]| unimplemented!(),
    );
    assert_eq!(function.ty(&store), function_type);

    // Using array signature
    let function_type = ([Type::V128], [Type::I32, Type::F32, Type::F64]);
    let function = Function::new(
        &mut store,
        function_type,
        |_values: &[Value]| unimplemented!(),
    );
    assert_eq!(function.ty(&store).params(), [Type::V128]);
    assert_eq!(
        function.ty(&store).results(),
        [Type::I32, Type::F32, Type::F64]
    );
}

#[wasm_bindgen_test]
fn function_new_dynamic_env() {
    let mut store = Store::default();
    #[derive(Clone)]
    struct MyEnv {}
    let my_env = MyEnv {};
    let env = FunctionEnv::new(&mut store, my_env);

    // Using &FunctionType signature
    let function_type = FunctionType::new(vec![], vec![]);
    let function = Function::new_with_env(
        &mut store,
        &env,
        &function_type,
        |_env: FunctionEnvMut<MyEnv>, _values: &[Value]| unimplemented!(),
    );
    assert_eq!(function.ty(&store), function_type);
    let function_type = FunctionType::new(vec![Type::I32], vec![]);
    let function = Function::new_with_env(
        &mut store,
        &env,
        &function_type,
        |_env: FunctionEnvMut<MyEnv>, _values: &[Value]| unimplemented!(),
    );
    assert_eq!(function.ty(&store), function_type);
    let function_type = FunctionType::new(vec![Type::I32, Type::I64, Type::F32, Type::F64], vec![]);
    let function = Function::new_with_env(
        &mut store,
        &env,
        &function_type,
        |_env: FunctionEnvMut<MyEnv>, _values: &[Value]| unimplemented!(),
    );
    assert_eq!(function.ty(&store), function_type);
    let function_type = FunctionType::new(vec![], vec![Type::I32]);
    let function = Function::new_with_env(
        &mut store,
        &env,
        &function_type,
        |_env: FunctionEnvMut<MyEnv>, _values: &[Value]| unimplemented!(),
    );
    assert_eq!(function.ty(&store), function_type);
    let function_type = FunctionType::new(vec![], vec![Type::I32, Type::I64, Type::F32, Type::F64]);
    let function = Function::new_with_env(
        &mut store,
        &env,
        &function_type,
        |_env: FunctionEnvMut<MyEnv>, _values: &[Value]| unimplemented!(),
    );
    assert_eq!(function.ty(&store), function_type);

    // Using array signature
    let function_type = ([Type::V128], [Type::I32, Type::F32, Type::F64]);
    let function = Function::new_with_env(
        &mut store,
        &env,
        function_type,
        |_env: FunctionEnvMut<MyEnv>, _values: &[Value]| unimplemented!(),
    );
    assert_eq!(function.ty(&store).params(), [Type::V128]);
    assert_eq!(
        function.ty(&store).results(),
        [Type::I32, Type::F32, Type::F64]
    );
}
