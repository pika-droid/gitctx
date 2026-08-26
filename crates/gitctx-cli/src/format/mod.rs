use crate::commands::blame::BlameHunkInfo;
use crate::commands::doctor::DoctorCheck;
use comfy_table::{presets::UTF8_FULL, Attribute, Cell, Color, Table};
use gitctx_query::chain::LineageChain;
use gitctx_query::cochange::{CoChangeMetrics, ExtractionStats, ImpactResult};
use gitctx_query::retrieval::RetrievedFact;
use std::time::Duration;

pub fn format_doctor(checks: &[DoctorCheck]) {
    let mut table = Table::new();
    table.load_preset(UTF8_FULL).set_header(vec![
        Cell::new("Check").add_attribute(Attribute::Bold),
        Cell::new("Status").add_attribute(Attribute::Bold),
        Cell::new("Details").add_attribute(Attribute::Bold),
    ]);

    for check in checks {
        let status_cell = match check.status.as_str() {
            "PASS" => Cell::new("PASS")
                .fg(Color::Green)
                .add_attribute(Attribute::Bold),
            "WARN" => Cell::new("WARN")
                .fg(Color::Yellow)
                .add_attribute(Attribute::Bold),
            _ => Cell::new("FAIL")
                .fg(Color::Red)
                .add_attribute(Attribute::Bold),
        };

        table.add_row(vec![
            Cell::new(&check.name),
            status_cell,
            Cell::new(&check.details),
        ]);
    }

    println!("\nGitCtx Doctor Diagnostic\n");
    println!("{table}");
}

pub fn format_evolution(chain: &LineageChain) {
    let mut table = Table::new();
    table.load_preset(UTF8_FULL).set_header(vec![
        Cell::new("Commit")
            .add_attribute(Attribute::Bold)
            .fg(Color::Cyan),
        Cell::new("Author").add_attribute(Attribute::Bold),
        Cell::new("Date")
            .add_attribute(Attribute::Bold)
            .fg(Color::Yellow),
        Cell::new("Change").add_attribute(Attribute::Bold),
        Cell::new("Lines").add_attribute(Attribute::Bold),
        Cell::new("Message").add_attribute(Attribute::Bold),
    ]);

    for node in &chain.nodes {
        let short_hash = if node.touch.commit_hash.len() > 8 {
            &node.touch.commit_hash[..8]
        } else {
            &node.touch.commit_hash
        };
        let first_line_msg = node.commit_message.lines().next().unwrap_or("").trim();
        let truncated_msg = if first_line_msg.len() > 40 {
            format!("{}...", &first_line_msg[..37])
        } else {
            first_line_msg.to_string()
        };

        let change_cell = match node.touch.change_type.to_lowercase().as_str() {
            "insert" | "add" => Cell::new(&node.touch.change_type).fg(Color::Green),
            "delete" | "remove" => Cell::new(&node.touch.change_type).fg(Color::Red),
            _ => Cell::new(&node.touch.change_type).fg(Color::Yellow),
        };

        table.add_row(vec![
            Cell::new(short_hash).fg(Color::Cyan),
            Cell::new(&node.commit_author),
            Cell::new(node.commit_authored_at.to_string()),
            change_cell,
            Cell::new(format!("{}:{}", node.touch.start_line, node.touch.end_line)),
            Cell::new(truncated_msg),
        ]);
    }

    println!(
        "\nEvolution for {} ({} touches)\n",
        chain.current_qualified_name,
        chain.nodes.len()
    );
    println!("{table}");
}

pub fn format_search(query: &str, results: &[RetrievedFact], duration: Duration) {
    if results.is_empty() {
        println!("\nNo results found for query: \"{query}\"");
        return;
    }

    let mut table = Table::new();
    table.load_preset(UTF8_FULL).set_header(vec![
        Cell::new("Rank").add_attribute(Attribute::Bold),
        Cell::new("Score")
            .add_attribute(Attribute::Bold)
            .fg(Color::Green),
        Cell::new("Function / Symbol")
            .add_attribute(Attribute::Bold)
            .fg(Color::Cyan),
        Cell::new("Path : Lines").add_attribute(Attribute::Bold),
        Cell::new("Commit")
            .add_attribute(Attribute::Bold)
            .fg(Color::Yellow),
        Cell::new("Rule").add_attribute(Attribute::Bold),
    ]);

    for (i, r) in results.iter().enumerate() {
        let short_hash = if r.commit_hash.len() > 8 {
            &r.commit_hash[..8]
        } else {
            &r.commit_hash
        };
        table.add_row(vec![
            Cell::new(format!("#{}", i + 1)),
            Cell::new(format!("{:.4}", r.score)).fg(Color::Green),
            Cell::new(&r.qualified_name).fg(Color::Cyan),
            Cell::new(format!(
                "{} @ {}:{}",
                r.path, r.citation.start_line, r.citation.end_line
            )),
            Cell::new(short_hash).fg(Color::Yellow),
            Cell::new(&r.citation.match_rule),
        ]);
    }

    println!(
        "\nSearch Results for \"{query}\" ({} matches, {duration:?})\n",
        results.len()
    );
    println!("{table}");
}

pub fn format_impact_single(target: &str, impact: &ImpactResult) {
    println!("\nImpact Analysis for {target}");
    println!("Risk Score:        {:.2}", impact.risk_score);
    println!("Follow-up Fixes:   {}", impact.followup_fix_count);

    if impact.coupled_entities.is_empty() {
        println!("\nNo strongly coupled entities found.");
    } else {
        let mut table = Table::new();
        table.load_preset(UTF8_FULL).set_header(vec![
            Cell::new("Coupled Entity")
                .add_attribute(Attribute::Bold)
                .fg(Color::Cyan),
            Cell::new("Confidence")
                .add_attribute(Attribute::Bold)
                .fg(Color::Green),
            Cell::new("Lift")
                .add_attribute(Attribute::Bold)
                .fg(Color::Yellow),
            Cell::new("Support").add_attribute(Attribute::Bold),
        ]);

        for c in &impact.coupled_entities {
            table.add_row(vec![
                Cell::new(&c.name).fg(Color::Cyan),
                Cell::new(format!("{:.2}", c.confidence)).fg(Color::Green),
                Cell::new(format!("{:.2}", c.lift)).fg(Color::Yellow),
                Cell::new(c.support.to_string()),
            ]);
        }
        println!("\nHighly Coupled Entities:\n{table}");
    }
}

pub fn format_impact_global(
    stats: &ExtractionStats,
    file_metrics: &[CoChangeMetrics],
    func_metrics: &[CoChangeMetrics],
) {
    println!("\nCo-Change Extraction Stats:");
    println!("  Total Commits:      {}", stats.total_commits);
    println!("  Qualifying Commits: {}", stats.qualifying_commits);
    println!(
        "  Filtered:           {} by size (>25 files), {} by bots",
        stats.size_filtered, stats.bot_filtered
    );

    if !file_metrics.is_empty() {
        let mut table = Table::new();
        table.load_preset(UTF8_FULL).set_header(vec![
            Cell::new("Entity A")
                .add_attribute(Attribute::Bold)
                .fg(Color::Cyan),
            Cell::new("Entity B")
                .add_attribute(Attribute::Bold)
                .fg(Color::Cyan),
            Cell::new("Support").add_attribute(Attribute::Bold),
            Cell::new("Lift")
                .add_attribute(Attribute::Bold)
                .fg(Color::Yellow),
        ]);
        for m in file_metrics.iter().take(5) {
            table.add_row(vec![
                Cell::new(&m.entity_a),
                Cell::new(&m.entity_b),
                Cell::new(m.support.to_string()),
                Cell::new(format!("{:.2}", m.lift)).fg(Color::Yellow),
            ]);
        }
        println!("\nTop File Couplings:\n{table}");
    }

    if !func_metrics.is_empty() {
        let mut table = Table::new();
        table.load_preset(UTF8_FULL).set_header(vec![
            Cell::new("Function A")
                .add_attribute(Attribute::Bold)
                .fg(Color::Cyan),
            Cell::new("Function B")
                .add_attribute(Attribute::Bold)
                .fg(Color::Cyan),
            Cell::new("Support").add_attribute(Attribute::Bold),
            Cell::new("Lift")
                .add_attribute(Attribute::Bold)
                .fg(Color::Yellow),
        ]);
        for m in func_metrics.iter().take(5) {
            table.add_row(vec![
                Cell::new(&m.entity_a),
                Cell::new(&m.entity_b),
                Cell::new(m.support.to_string()),
                Cell::new(format!("{:.2}", m.lift)).fg(Color::Yellow),
            ]);
        }
        println!("\nTop Function Couplings:\n{table}");
    }
}

pub fn format_show(
    hash: &str,
    author: &str,
    email: &str,
    authored_at: i64,
    message: &str,
    parents: &str,
    touches: &[(String, String, String, i64, i64)],
) {
    println!("Commit:  {hash}");
    println!("Author:  {author} <{email}>");
    println!("Date:    {authored_at}");
    if !parents.is_empty() {
        println!("Parents: {parents}");
    }
    println!("\n    {}", message.replace('\n', "\n    "));

    if !touches.is_empty() {
        let mut table = Table::new();
        table.load_preset(UTF8_FULL).set_header(vec![
            Cell::new("Change").add_attribute(Attribute::Bold),
            Cell::new("Qualified Name")
                .add_attribute(Attribute::Bold)
                .fg(Color::Cyan),
            Cell::new("Path").add_attribute(Attribute::Bold),
            Cell::new("Lines").add_attribute(Attribute::Bold),
        ]);

        for (path, qual, change, start, end) in touches {
            let change_cell = match change.to_lowercase().as_str() {
                "insert" | "add" => Cell::new(change.to_uppercase()).fg(Color::Green),
                "delete" | "remove" => Cell::new(change.to_uppercase()).fg(Color::Red),
                _ => Cell::new(change.to_uppercase()).fg(Color::Yellow),
            };
            table.add_row(vec![
                change_cell,
                Cell::new(qual).fg(Color::Cyan),
                Cell::new(path),
                Cell::new(format!("{start}:{end}")),
            ]);
        }
        println!("\nFunctions Touched ({}):\n{table}", touches.len());
    }
}

pub fn format_blame(file_path: &str, hunks: &[BlameHunkInfo]) {
    let mut table = Table::new();
    table.load_preset(UTF8_FULL).set_header(vec![
        Cell::new("Commit")
            .add_attribute(Attribute::Bold)
            .fg(Color::Cyan),
        Cell::new("Author").add_attribute(Attribute::Bold),
        Cell::new("Line Range")
            .add_attribute(Attribute::Bold)
            .fg(Color::Yellow),
        Cell::new("Lines").add_attribute(Attribute::Bold),
    ]);

    for hunk in hunks {
        let short_hash = if hunk.commit.len() > 8 {
            &hunk.commit[..8]
        } else {
            &hunk.commit
        };
        let line_range = if hunk.line_count > 1 {
            format!(
                "{}-{}",
                hunk.start_line,
                hunk.start_line + hunk.line_count - 1
            )
        } else {
            hunk.start_line.to_string()
        };

        table.add_row(vec![
            Cell::new(short_hash).fg(Color::Cyan),
            Cell::new(&hunk.author),
            Cell::new(line_range).fg(Color::Yellow),
            Cell::new(hunk.line_count.to_string()),
        ]);
    }

    println!("\nBlame for {file_path}:\n{table}");
}
