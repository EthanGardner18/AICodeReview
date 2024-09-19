use std::fs::File;
use std::io::Write;

fn main() -> std::io::Result<()> {
    let dot_content = r#"
digraph G {
    rankdir=LR;
    node [shape=box];

    // Define actors
    Actor1 [label="Actor 1: API Connection"];
    Actor2 [label="Actor 2: Function Finder"];
    Actor3 [label="Actor 3: Send/Receive LLM Response"];
    Actor4 [label="Actor 4: Interpret Response"];
    Actor5 [label="Actor 5: Save as .txt"];

    // Define connections
    Actor1 -> Actor3 [label="Establish connection"];
    Actor2 -> Actor3 [label="Find Function"];
    Actor3 -> Actor4 [label="Send Response"];
    Actor4 -> Actor5 [label="Interpret and Save"];
}
"#;

    // Create and write to a .dot file
    let mut file = File::create("actors_graph.dot")?;
    file.write_all(dot_content.as_bytes())?;

    println!("DOT file 'actors_graph.dot' generated successfully.");
    Ok(())
}
