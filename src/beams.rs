use std::env;
use tokio::fs;
use tokio::io::{self, AsyncBufReadExt, BufReader};

/// Represents a completed beam solution.
#[derive(Debug, Clone)]
struct BeamPlan {
    total: usize,
    welds: usize,
    used_offcuts: Vec<usize>,
}

/// Represents a beam requirement specification.
#[derive(Debug, Clone)]
struct BeamRequires {
    size: usize,
    welds: Vec<usize>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: {} <beam_requirements_file> <offcuts_file>", args[0]);
        return Err("Invalid arguments".into());
    }

    let (beam_file, offcuts_file) = (&args[1], &args[2]);

    // Load files concurrently
    let (beam_requirements, offcuts) = tokio::try_join!(
        load_beam_requirements(beam_file),
        load_offcuts(offcuts_file)
    )?;

    // Process beams
    process_beams(beam_requirements, offcuts).await;

    Ok(())
}

/// Load beam requirements from file asynchronously
async fn load_beam_requirements(path: &str) -> io::Result<Vec<BeamRequires>> {
    let file = fs::File::open(path).await?;
    let reader = BufReader::new(file);
    let mut lines = reader.lines();
    let mut requirements = Vec::new();

    while let Some(line) = lines.next_line().await? {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let numbers: Vec<usize> = line
            .split_whitespace()
            .filter_map(|s| s.parse::<usize>().ok())
            .collect();

        if let Some((&size, welds)) = numbers.split_first() {
            requirements.push(BeamRequires {
                size,
                welds: welds.to_vec(),
            });
        }
    }

    // Sort by size descending for better greedy allocation
    requirements.sort_unstable_by(|a, b| b.size.cmp(&a.size));
    Ok(requirements)
}

/// Load offcuts from file asynchronously
async fn load_offcuts(path: &str) -> io::Result<Vec<usize>> {
    let content = fs::read_to_string(path).await?;
    let mut offcuts: Vec<usize> = content
        .split_whitespace()
        .filter_map(|s| s.parse::<usize>().ok())
        .collect();

    // Sort descending for optimal greedy selection
    offcuts.sort_unstable_by(|a, b| b.cmp(a));
    Ok(offcuts)
}

/// Process all beam requirements and print results
async fn process_beams(requirements: Vec<BeamRequires>, mut offcuts: Vec<usize>) {
    for (idx, beam) in requirements.iter().enumerate() {
        println!("{} mm", beam.size);

        for &max_welds in beam.welds.iter() {
            match find_combinations(&mut offcuts, beam.size, max_welds) {
                Some(plan) => {
                    print_solution(&plan, beam.size, max_welds);
                }
                None => {
                    println!("{} mm with {} weld - not found", beam.size, max_welds);
                }
            }
        }

        // Add blank line between beams
        if idx < requirements.len() - 1 {
            println!();
        }
    }
}

/// Print a beam solution in the format shown in README
fn print_solution(plan: &BeamPlan, target: usize, max_welds: usize) {
    println!(
        "{} mm, max {} weld: BeamPlan {{ total: {}, welds: {}, used_offcuts: {:?} }}",
        target, max_welds, plan.total, plan.welds, plan.used_offcuts
    );
}

/// Find optimal combination of offcuts using improved greedy algorithm
fn find_combinations(
    offcuts: &mut Vec<usize>,
    target_length: usize,
    max_welds: usize,
) -> Option<BeamPlan> {
    let mut used_offcuts = Vec::new();
    let mut total = 0;
    let max_pieces = max_welds + 1;

    // Try to build the beam with up to max_pieces
    for _ in 0..max_pieces {
        let remaining = target_length.saturating_sub(total);

        // Strategy 1: Try to complete the beam with a single piece
        if let Some((index, &offcut)) = offcuts
            .iter()
            .enumerate()
            .filter(|&(_, &offcut)| offcut >= remaining)
            .min_by_key(|&(_, &offcut)| offcut)
        {
            used_offcuts.push(offcut);
            offcuts.remove(index);
            total += offcut;

            // Successfully completed
            return Some(BeamPlan {
                total,
                welds: used_offcuts.len() - 1,
                used_offcuts,
            });
        }

        if let Some((index, &offcut)) = offcuts
            .iter()
            .enumerate()
            .filter(|&(_, &offcut)| offcut <= remaining)
            .max_by_key(|&(_, &offcut)| offcut)
        {
            used_offcuts.push(offcut);
            offcuts.remove(index);
            total += offcut;
        } else {
            // No piece fits, cannot continue
            break;
        }
    }

    // Failed to reach target - restore offcuts
    for offcut in used_offcuts {
        offcuts.push(offcut);
    }
    offcuts.sort_unstable_by(|a, b| b.cmp(a));

    None
}
