use super::LanguageError;
pub(super) fn run(
    source: &str,
    input: &[u8],
    budget: usize,
) -> Result<(Vec<u8>, usize), LanguageError> {
    let mut program = Vec::new();
    for c in source.chars() {
        if c.is_whitespace() {
            continue;
        }
        if !"><+-.,[]".contains(c) {
            return Err(LanguageError::TapeSyntax);
        }
        program.push(c)
    }
    let mut pairs = vec![0; program.len()];
    let mut stack = Vec::new();
    for (i, c) in program.iter().enumerate() {
        match c {
            '[' => {
                stack.push(i);
                if stack.len() > 16 {
                    return Err(LanguageError::DepthLimit);
                }
            }
            ']' => {
                let start = stack.pop().ok_or(LanguageError::TapeSyntax)?;
                pairs[start] = i;
                pairs[i] = start
            }
            _ => {}
        }
    }
    if !stack.is_empty() {
        return Err(LanguageError::TapeSyntax);
    }
    let mut cells = [0_u8; 1024];
    let mut ptr = 0usize;
    let mut ip = 0usize;
    let mut read = 0;
    let mut steps = 0;
    let mut output = Vec::new();
    while ip < program.len() {
        if steps >= budget {
            return Err(LanguageError::StepBudget);
        }
        steps += 1;
        match program[ip] {
            '>' => ptr = (ptr + 1) % 1024,
            '<' => ptr = (ptr + 1023) % 1024,
            '+' => cells[ptr] = cells[ptr].wrapping_add(1),
            '-' => cells[ptr] = cells[ptr].wrapping_sub(1),
            '.' => {
                if output.len() >= 1024 {
                    return Err(LanguageError::OutputBudget);
                }
                output.push(cells[ptr])
            }
            ',' => {
                cells[ptr] = input.get(read).copied().unwrap_or(0);
                read = read.saturating_add(1)
            }
            '[' if cells[ptr] == 0 => ip = pairs[ip],
            ']' if cells[ptr] != 0 => ip = pairs[ip],
            _ => {}
        }
        ip += 1;
    }
    Ok((output, steps))
}
