#[test]
fn test_loop_and_break() {
    let state = Arc::new(Mutex::new(ChatState::new()));
    let mut env = Environment::new();
    let env_arc = Arc::new(Mutex::new(env.clone()));
    builtins::populate(&mut env, Arc::clone(&state), env_arc);

    // loop with break
    let input = "let x = 0\nloop { x = x + 1; if x == 5 then break x else () }";
    let expr = parse_script(input).unwrap();
    let result = eval_expr(&expr, &mut env, state).unwrap();
    assert_eq!(result.display(), "5");
}

#[test]
fn test_list_comprehension() {
    let state = Arc::new(Mutex::new(ChatState::new()));
    let mut env = Environment::new();
    let env_arc = Arc::new(Mutex::new(env.clone()));
    builtins::populate(&mut env, Arc::clone(&state), env_arc);

    let expr = parse_expression("[x * 2 for x in [1,2,3] if x > 1]").unwrap();
    let result = eval_expr(&expr, &mut env, state).unwrap();
    assert_eq!(result.display(), "[4, 6]");
}

#[test]
fn test_load_and_del() {
    use std::fs;
    use std::io::Write;
    let state = Arc::new(Mutex::new(ChatState::new()));
    let mut env = Environment::new();
    let env_arc = Arc::new(Mutex::new(env.clone()));
    builtins::populate(&mut env, Arc::clone(&state), env_arc);

    // Create temp file
    let temp_path = "test_load.plic";
    let content = "let x = 42";
    fs::write(temp_path, content).unwrap();

    let expr = parse_script(&format!("load \"{}\"", temp_path)).unwrap();
    eval_expr(&expr, &mut env, Arc::clone(&state)).unwrap();
    // Now x should be defined
    let expr2 = parse_expression("x").unwrap();
    let result = eval_expr(&expr2, &mut env, Arc::clone(&state)).unwrap();
    assert_eq!(result.display(), "42");

    // Test del
    let expr3 = parse_expression("del \"x\"").unwrap();
    eval_expr(&expr3, &mut env, Arc::clone(&state)).unwrap();
    let expr4 = parse_expression("x").unwrap();
    let result4 = eval_expr(&expr4, &mut env, state);
    assert!(result4.is_err());

    fs::remove_file(temp_path).ok();
}
