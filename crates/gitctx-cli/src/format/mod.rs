use comfy_table::{Table, Cell, Color, Attribute, presets::UTF8_FULL};

#[allow(dead_code)]
pub fn format_evolution(identifier: &str, results: &[String]) {
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_header(vec![
            Cell::new("Commit").add_attribute(Attribute::Bold).fg(Color::Cyan),
            Cell::new("Date").add_attribute(Attribute::Bold).fg(Color::Yellow),
            Cell::new("Author").add_attribute(Attribute::Bold),
            Cell::new("Message").add_attribute(Attribute::Bold),
        ]);

    for res in results {
        table.add_row(vec![
            Cell::new(res).fg(Color::Green),
            Cell::new("2026-08-23"),
            Cell::new("gitctx"),
            Cell::new("Example commit"),
        ]);
    }

    println!("\nEvolution of {}\n", identifier);
    println!("{table}");
}

pub fn format_doctor(health: bool, schema: bool, config: bool) {
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_header(vec![
            Cell::new("Check").add_attribute(Attribute::Bold),
            Cell::new("Status").add_attribute(Attribute::Bold),
        ]);

    let pass_cell = Cell::new("PASS").fg(Color::Green).add_attribute(Attribute::Bold);
    let fail_cell = Cell::new("FAIL").fg(Color::Red).add_attribute(Attribute::Bold);

    table.add_row(vec![Cell::new("SQLite Index Health"), if health { pass_cell.clone() } else { fail_cell.clone() }]);
    table.add_row(vec![Cell::new("Schema Version Compatibility"), if schema { pass_cell.clone() } else { fail_cell.clone() }]);
    table.add_row(vec![Cell::new("Config Hash Match"), if config { pass_cell.clone() } else { fail_cell.clone() }]);

    println!("\nGitCtx Doctor Diagnostic\n");
    println!("{table}");
}

