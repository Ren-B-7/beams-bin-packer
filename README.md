# Beams weld calculator

A high-performance Rust application for optimizing beam construction from offcut materials. Solves the problem of combining available offcuts to create required beam lengths with minimal welds.

### Why

Given:

- A set of **required beams** (target lengths with maximum weld constraints)
- A pool of **available offcuts** (various lengths of leftover material)

Find:

- Combinations of offcuts that can be welded together to meet or exceed each target length
- Solutions that respect the maximum number of welds allowed

## Installation

### Build from Source

```bash
cargo build --release
install -m 755 target/release/beams $(HOME)/.local/bin/
```

## Usage

```bash
beams <beam_requirements_file> <offcuts_file>
```

## Input File Formats

### Beam Requirements File

Each line specifies one required beam:

```ini
<target_length> <max_welds_1> <max_welds_2> ...
```

**Example** (`beams.txt`):

```ini
5000 1 2 3
3500 1 2
4200 2 3 4
```

This means:

- Build a 5000mm beam with solutions for 1, 2, and 3 welds maximum
- Build a 3500mm beam with solutions for 1 and 2 welds maximum
- Build a 4200mm beam with solutions for 2, 3, and 4 welds maximum

### Offcuts File

Space-separated list of available offcut lengths:

**Example** (`offcuts.txt`):

```ini
2500 3000 1500 2200 1800 4000 1200 2700 3500 1600
```

## Output Format

```ini
5000 mm
5000 mm, max 1 weld: BeamPlan { total: 5200, welds: 1, used_offcuts: [3000, 2200] }
5000 mm, max 2 weld: BeamPlan { total: 5000, welds: 2, used_offcuts: [2500, 1500, 1000] }
5000 mm with 3 weld - not found

3500 mm
3500 mm, max 1 weld: BeamPlan { total: 3500, welds: 1, used_offcuts: [2000, 1500] }
```

Each solution shows:

- `total` - Actual combined length (may exceed target)
- `welds` - Number of welds used
- `used_offcuts` - List of offcut lengths that were combined

## Algorithm

The greedy algorithm works as follows:

1. **Sort offcuts** in descending order (largest first)
2. For each required beam and weld count:
   - Try to find the **smallest offcut that completes** the beam (>= target length)
   - If not possible, take the **largest offcut that fits** (<= remaining length)
   - Repeat until target is met or max welds reached
3. **Remove used offcuts** from the available pool
4. If no solution found, **return offcuts** to the pool

## Dependencies

```toml
tokio = { version = "1.42", features = ["full"] }
```
