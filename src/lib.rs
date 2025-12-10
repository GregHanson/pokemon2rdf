use indicatif::{MultiProgress, ProgressStyle};
use oxrdf::vocab;
use oxrdf::{NamedNode, Triple};
use std::error::Error;
use std::io::{BufWriter, Write};
use std::sync::Arc;
use tokio::sync::mpsc;

pub(crate) mod collections;
use crate::collections::*;

// Pokemon ontology vocabulary namespace
static POKE: &str = "http://purl.org/pokemon/ontology#";

// Standard vocabulary namespaces for alignment
static POKEMONKG: &str = "https://pokemonkg.org/ontology#";
static SCHEMA: &str = "https://schema.org/";
// Reserved for future use:
// static FOAF: &'static str = "http://xmlns.com/foaf/0.1/";
// static DCTERMS: &'static str = "http://purl.org/dc/terms/";

// TODO can we add any of this to enhance the triples being built?
// example: https://github.com/MarErius/Pokeapp/blob/main/MAINPROGRAM.py

pub async fn build_graph() -> Result<(), Box<dyn Error + Send + Sync>> {
    let client = rustemon::client::RustemonClient::default();

    // Generate output filename with current date: pokemon-YYYY-MM-DD.nt
    let now = chrono::Local::now();
    let filename = format!("pokemon-{}.nt", now.format("%Y-%m-%d"));

    println!("Writing output to: {}", filename);

    // Create/overwrite the output file in current working directory
    let output_file = std::fs::File::create(&filename)
        .map_err(|e| format!("Failed to create output file {}: {}", filename, e))?;

    let m = MultiProgress::new();
    let _sty = ProgressStyle::with_template(
        "[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}",
    )
    .unwrap()
    .progress_chars("##-");

    // Create an unbounded channel for sending triples from workers to writer
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();

    // Spawn a dedicated writer task that consumes from the channel
    let mut output_file = output_file;

    let writer_handle = tokio::spawn(async move {
        let mut writer = BufWriter::new(&mut output_file);
        while let Some(line) = rx.recv().await {
            if let Err(e) = writeln!(writer, "{}", line) {
                eprintln!("Error writing to output: {}", e);
                return Err::<(), Box<dyn Error + Send + Sync>>(e.into());
            }
        }
        writer.flush().map_err(|e| {
            eprintln!("Error flushing output: {}", e);
            e
        })?;
        Ok(())
    });

    // Wrap client in Arc for sharing across tasks
    let client = Arc::new(client);

    // Spawn all conversion tasks concurrently - each sends triples to the channel
    // let mut handles = vec![];

    abilities::ability_to_nt(m.clone(), client.clone(), tx.clone()).await?;
    damage_class::damage_class_to_nt(m.clone(), client.clone(), tx.clone()).await?;
    egg_groups::egg_group_to_nt(m.clone(), client.clone(), tx.clone()).await?;
    evolutions_chains::evolution_chain_to_nt(m.clone(), client.clone(), tx.clone()).await?;
    forms::form_to_nt(m.clone(), client.clone(), tx.clone()).await?;
    generations::generation_to_nt(m.clone(), client.clone(), tx.clone()).await?;
    growth_rates::growth_rate_to_nt(m.clone(), client.clone(), tx.clone()).await?;
    habitats::habitat_to_nt(m.clone(), client.clone(), tx.clone()).await?;
    locations::location_to_nt(m.clone(), client.clone(), tx.clone()).await?;
    moves::move_to_nt(m.clone(), client.clone(), tx.clone()).await?;
    move_target::move_target_to_nt(m.clone(), client.clone(), tx.clone()).await?;
    pal_park::pal_park_area_to_nt(m.clone(), client.clone(), tx.clone()).await?;
    pokemon::pokemon_to_nt(m.clone(), client.clone(), tx.clone()).await?;
    regions::region_to_nt(m.clone(), client.clone(), tx.clone()).await?;
    shapes::shape_to_nt(m.clone(), client.clone(), tx.clone()).await?;
    species::species_to_nt(m.clone(), client.clone(), tx.clone()).await?;
    poke_types::type_to_nt(m.clone(), client.clone(), tx.clone()).await?;
    stats::stat_to_nt(m.clone(), client.clone(), tx.clone()).await?;
    natures::nature_to_nt(m.clone(), client.clone(), tx.clone()).await?;
    triggers::trigger_to_nt(m.clone(), client.clone(), tx.clone()).await?;
    pokedex::pokedex_to_nt(m.clone(), client.clone(), tx.clone()).await?;
    items::item_to_nt(m.clone(), client.clone(), tx.clone()).await?;

    // // Wait for all worker tasks to complete
    // for handle in handles {
    //     handle.await??;
    // }

    // Drop the sender to signal the writer that no more data is coming
    drop(tx);

    // Wait for the writer to finish processing all messages
    writer_handle.await??;

    // TODO BerryFirmness
    // TODO Berry Flavor
    // TODO Berry
    // TODO ContestType
    // TODO EncounterCondition
    // TODO EncounterConditionValue
    // TODO MoveLearnMethod
    // TODO Version
    // TODO ItemFlingEffect
    // TODO ItemAttribute
    // TODO ItemCategory
    // TODO ItemPocket
    // TODO ItemCategory
    // TODO VersionEncounterDetail
    // TODO EncounterMethod
    // TODO MoveAilment
    // TODO MoveCategory
    // TODO VersionGroup
    // TODO PokeathonStat
    // TODO MoveBattleStyle
    // TODO LocationAreaEncounter

    Ok(())
}

// Helper functions to create triples
fn create_type_triple(
    subject: impl Into<oxrdf::NamedOrBlankNode>,
    class_name: &str,
) -> Result<Triple, Box<dyn Error + Send + Sync>> {
    // Use pokemonkg ontology for known classes, POKE namespace only for novel concepts
    let namespace = match class_name {
        "Species" | "Ability" | "Move" | "Type" | "Region" | "Habitat" | "EggGroup"
        | "Generation" | "Shape" | "Pokémon" => POKEMONKG,
        _ => POKE,
    };

    Ok(Triple {
        subject: subject.into(),
        predicate: vocab::rdf::TYPE.into(),
        object: NamedNode::new(format!("{}{}", namespace, class_name))?.into(),
    })
}

#[cfg(test)]
mod tests {

    use super::*;

    #[tokio::test]
    async fn test_rdf() {
        assert!((build_graph().await).is_ok())
    }
}
