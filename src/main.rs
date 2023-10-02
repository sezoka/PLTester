use std::{self, io::Write, process::Stdio, str::Chars};

struct Parser<'a> {
    chars: Chars<'a>,
    line: usize,
}

struct Test {
    name: String,
    input: String,
    expected: String,
    line: usize,
}

struct TestsData {
    tests: Vec<Test>,
    command: String,
}

fn main() {
    let mut arg_iter = std::env::args().skip(2);

    // TODO(sezoka): add option to escape output strings for error messages
    // e.g. '  ' -> '\t'

    while let Some(arg) = arg_iter.next() {
        let test_path = arg;
        parse_and_run(&test_path);
        break;
    }
}

fn parse_and_run(path: &str) -> Option<()> {
    let file = read_file(path)?;
    let tests_data = parse(file)?;
    run_tests(tests_data)?;
    remove_temp_files();
    return Some(());
}

fn remove_temp_files() {
    std::fs::remove_dir_all("/tmp/pltest").unwrap();
}

fn run_tests(tests_data: TestsData) -> Option<()> {
    if 1 < tests_data.tests.len() {
        println!("RUNNING {} TESTS:", tests_data.tests.len());
    } else {
        println!("RUNNING {} TEST:", tests_data.tests.len());
    }
    println!();

    std::fs::create_dir_all("/tmp/pltest").unwrap();

    let mut failed_tests = Vec::new();

    for test in tests_data.tests.iter() {
        if run_test(&test, &tests_data).is_none() {
            failed_tests.push((&test.name, test.line));
        }
    }

    println!();
    if failed_tests.is_empty() {
        println!("All tests successfully completed!");
    } else {
        println!("FAILED TESTS:");
        for (test_name, line) in &failed_tests {
            println!("{} on line {}", test_name, line);
        }
        println!(
            "\nSuccessfully completed {} out of {} tests.",
            tests_data.tests.len() - failed_tests.len(),
            tests_data.tests.len()
        );
    }
    println!();

    Some(())
}

fn run_test(t: &Test, td: &TestsData) -> Option<()> {
    let test_file_name = t
        .name
        .chars()
        .map(|c| {
            if c.is_whitespace() {
                '_'
            } else {
                c.to_ascii_lowercase()
            }
        })
        .collect::<String>();
    let test_file_path = format!("/tmp/pltest/{}", &test_file_name);
    let cmd_str = &td.command;

    let mut file = match std::fs::File::create(&test_file_path) {
        Ok(file) => file,
        Err(err) => {
            eprintln!(
                "Error: can't create test file at '{test_file_path}', {:?}",
                err
            );
            return None;
        }
    };
    if let Err(_) = file.write_all(t.input.as_bytes()) {
        eprintln!("Error: can't write test input to temporary file at '{test_file_path}'");
    }

    let mut cmd = std::process::Command::new(&cmd_str);
    cmd.arg(&test_file_path);
    cmd.stdout(Stdio::piped());

    match cmd.spawn() {
        Ok(child) => match child.wait_with_output() {
            Ok(output) => {
                let result = unsafe { String::from_utf8_unchecked(output.stdout) };
                if !results_as_expected(&result, t) {
                    return None;
                }
            }
            Err(err) => {
                eprintln!("{}", err);
                return None;
            }
        },
        Err(err) => {
            eprintln!("Error: the '{cmd_str}' failed to run.\nReason: {:?}", err);
            return None;
        }
    }

    Some(())
}

fn results_as_expected(result: &str, t: &Test) -> bool {
    if result == t.expected {
        return true;
    }

    if t.expected.len() < result.len() {
        println!(
            "![{}]({}): output string length is greater than expected - {} vs {}",
            t.line,
            t.name,
            result.len(),
            t.expected.len()
        );
    } else if result.len() < t.expected.len() {
        println!(
            "![{}]({}): output string length is less than expected - {} vs {}",
            t.line,
            t.name,
            result.len(),
            t.expected.len()
        );
    }

    println!();
    print_difference(result, t);

    false
}

fn print_difference(result: &str, t: &Test) {
    println!(":got:\n\"{result}\"");
    println!(":expected:\n\"{}\"", t.expected);
}

fn read_file(path: &str) -> Option<String> {
    let file = std::fs::read_to_string(path);
    if file.is_err() {
        eprintln!("Error: failed to load test file using path {path}");
    }
    file.ok()
}

fn parse(file: String) -> Option<TestsData> {
    let mut p = Parser {
        line: 1,
        chars: file.chars(),
    };

    let mut tests_data = TestsData {
        tests: Vec::new(),
        command: String::new(),
    };

    skip_whitespaces(&mut p);
    tests_data.command = get_command()?;

    loop {
        skip_whitespaces(&mut p);
        if is_at_end(&mut p) {
            break;
        }
        let test = parse_test(&mut p)?;
        tests_data.tests.push(test);
    }

    Some(tests_data)
}

fn get_command() -> Option<String> {
    let mut args = std::env::args().skip(1);
    if let Some(cmd) = args.next() {
        return Some(cmd);
    }
    return None;
}

fn peek(p: &Parser) -> char {
    p.chars.clone().next().unwrap_or_default()
}

fn advance(p: &mut Parser) -> char {
    p.chars.next().unwrap_or_default()
}

fn skip_whitespaces(p: &mut Parser) {
    loop {
        match peek(p) {
            '\n' => p.line += 1,
            ' ' | '\t' | '\r' => {}
            _ => break,
        }
        advance(p);
    }
}

fn is_at_end(p: &mut Parser) -> bool {
    peek(p) == '\0'
}

fn _parse_command(p: &mut Parser) -> Option<String> {
    if !p.chars.as_str().starts_with("COMMAND:") {
        eprintln!("Error: expected 'COMMAND:' directive at top of the file");
        return None;
    }

    skip_str(p, "COMMAND:")?;

    let start = p.chars.as_str();
    while !is_at_end(p) && peek(p) != '\n' {
        advance(p);
    }

    if is_at_end(p) {
        eprintln!("Expected tests after the 'COMMAND:' directive");
        return None;
    }

    let command = get_substring(p, start);

    Some(command)
}

fn parse_test(p: &mut Parser) -> Option<Test> {
    if !p.chars.as_str().starts_with("TEST") {
        eprintln!("Error: expected 'TEST' directive");
        return None;
    }

    skip_str(p, "TEST");

    let mut test = Test {
        name: String::new(),
        line: p.line,
        input: String::new(),
        expected: String::new(),
    };

    test.name = parse_test_name(p)?;
    skip_whitespaces(p);
    let separator = parse_test_separator(p)?;
    test.input = parse_separated_test(p, &separator)?;
    test.expected = parse_separated_test(p, &separator)?;

    Some(test)
}

fn parse_test_name(p: &mut Parser) -> Option<String> {
    let start = p.chars.as_str();
    while !is_at_end(p) && peek(p) != ':' && peek(p) != '\n' {
        advance(p);
    }

    if is_at_end(p) {
        eprintln!("Error: test name should be on the same line with the 'TEST' directive");
        return None;
    }
    if peek(p) == '\n' {
        eprintln!("Error: expected test name after the 'TEST' directive");
        return None;
    }

    let name = get_substring(p, start);
    advance(p);
    Some(name)
}

fn parse_test_separator(p: &mut Parser) -> Option<String> {
    let start = p.chars.as_str();
    while !is_whitespace(peek(p)) {
        advance(p);
    }
    Some(get_substring(p, start))
}

fn is_whitespace(c: char) -> bool {
    matches!(c, ' ' | '\n' | '\t' | '\r')
}
fn get_substring(p: &Parser, start: &str) -> String {
    get_substr(p, start).to_string()
}

fn get_substr<'a>(p: &Parser, start: &'a str) -> &'a str {
    let len = start.len() - p.chars.as_str().len();
    start[0..len].trim_start()
}

fn parse_separated_test(p: &mut Parser, separator: &str) -> Option<String> {
    let first_char = separator.chars().next().unwrap_or_default();
    let start = p.chars.as_str();

    while !is_at_end(p) {
        if peek(p) == '\n' {
            p.line += 1;
        }

        if peek(p) == first_char {
            let substr = get_substr(p, start);

            let rest = p.chars.as_str();
            if &rest[0..separator.len()] == separator {
                skip_str(p, separator);
            }
            if peek(p) == '\n' {
                return Some(substr.to_string());
            }
        }

        advance(p);
    }

    None
}

fn skip_str(p: &mut Parser, str: &str) -> Option<()> {
    for _ in 0..str.len() {
        p.chars.next()?;
    }
    Some(())
}
