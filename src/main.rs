use pokemon2rdf::build_graph;

#[tokio::main]
async fn main() {
    println!("Hello, world!");
    match build_graph().await {
        Ok(_) => println!("Graph built successfully."),
        Err(e) => eprintln!("Error building graph: {}", e),
    };
}
