use std::env;
use std::sync::Arc;
use tokio::fs;
use tokio::io::{self, AsyncBufReadExt, BufReader};

/// Represents a completed beam solution.
#[derive(Debug, Clone)]
struct BeamPlan {
    total: usize,
    welds: usize,
    used_offcuts: Vec<usize>,
    waste: usize,
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
    let total_beams = requirements.len();
    let initial_offcuts = offcuts.len();
    let initial_material: usize = offcuts.iter().sum();

    println!("# Beam Welding Solutions\n");
    println!("## Input Summary\n");
    println!("- **Total beams required**: {}", total_beams);
    println!("- **Available offcuts**: {}", initial_offcuts);
    println!("- **Total material**: {} mm\n", initial_material);

    let mut solved_count = 0;
    let mut total_waste = 0;

    for (idx, beam) in requirements.iter().enumerate() {
        println!("## Beam {} - {} mm\n", idx + 1, beam.size);

        let mut beam_solved = false;

        for &max_welds in beam.welds.iter() {
            match find_combinations(&mut offcuts, beam.size, max_welds) {
                Some(plan) => {
                    beam_solved = true;
                    total_waste += plan.waste;
                    print_solution(&plan, beam.size, max_welds);
                }
                None => {
                    println!("❌ **Max {} weld{}** - No solution found\n", 
                        max_welds, if max_welds == 1 { "" } else { "s" });
                }
            }
        }

        if beam_solved {
            solved_count += 1;
        }

        // Add separator between beams
        if idx < total_beams - 1 {
            println!("---\n");
        }
    }

    // Summary statistics
    println!("\n## Summary\n");
    println!("- **Beams solved**: {}/{}", solved_count, total_beams);
    println!("- **Remaining offcuts**: {}", offcuts.len());
    println!("- **Total waste**: {} mm", total_waste);
    
    let remaining_material: usize = offcuts.iter().sum();
    println!("- **Remaining material**: {} mm", remaining_material);
    println!("- **Material efficiency**: {:.1}%", 
        ((initial_material - remaining_material) as f64 / initial_material as f64) * 100.0);
}

/// Print a beam solution in markdown format
fn print_solution(plan: &BeamPlan, target: usize, max_welds: usize) {
    let weld_text = if plan.welds == 1 { "weld" } else { "welds" };
    
    println!("✅ **Max {} {}** - Solution found", max_welds, 
        if max_welds == 1 { "weld" } else { "welds" });
    println!("- **Actual length**: {} mm", plan.total);
    println!("- **Welds used**: {}", plan.welds);
    println!("- **Waste**: {} mm ({:.1}%)", plan.waste, 
        (plan.waste as f64 / plan.total as f64) * 100.0);
    
    print!("- **Offcuts used**: ");
    for (i, &offcut) in plan.used_offcuts.iter().enumerate() {
        if i > 0 {
            print!(" + ");
        }
        print!("{} mm", offcut);
    }
    println!(" = {} mm\n", plan.total);
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
            let waste = total - target_length;
            return Some(BeamPlan {
                total,
                welds: used_offcuts.len() - 1,
                used_offcuts,
                waste,
            });
        }
        
        // Strategy 2: No piece can complete it, take the largest that fits
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
