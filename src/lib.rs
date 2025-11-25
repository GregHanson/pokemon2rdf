use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use oxrdf::vocab::xsd;
use oxrdf::{vocab, BlankNode};
use oxrdf::{Literal, NamedNode, NamedNodeRef, Triple};
use rustemon::client::RustemonClient;
use rustemon::model::evolution::ChainLink;
use rustemon::model::games::Generation;
use rustemon::model::locations::{Location, Region};
use rustemon::model::moves::{Move, MoveDamageClass, MoveTarget};
use rustemon::model::pokemon::PokemonType;
use rustemon::model::pokemon::{
    GrowthRate, Pokemon, PokemonForm, PokemonMove, PokemonSpecies, Type,
};
use rustemon::model::resource::NamedApiResource;
use rustemon::moves::move_;
use rustemon::Follow;
use std::error::Error;
use std::io::{BufWriter, Write};
use std::process::{Command, Stdio};
use std::sync::Arc;
use tempfile::{Builder, NamedTempFile, TempDir};
use tokio::sync::mpsc;

// Pokemon ontology vocabulary namespace
static POKE: &'static str = "http://purl.org/pokemon/ontology#";

// TODO can we add any of this to enhance the triples being built?
// example: https://github.com/MarErius/Pokeapp/blob/main/MAINPROGRAM.py
//
// static PAPI: &'static str ="https://pokeapi.co/api/v2/pokemon/";
// static BULB: &'static str = "https://bulbapedia.bulbagarden.net/wiki/";
// static DBR: &'static str = "http://dbpedia.org/resource/";
// static DUL: &'static str ="http://www.ontologydesignpatterns.org/ont/dul/DUL.owl#";
// static SCHEMA: &'static str = "http://schema.org/";
// static WD: &'static str = "http://www.wikidata.org/entity/";
// static RDFS: &'static str = "https://www.w3.org/2000/01/rdf-schema#";

pub async fn build_graph() -> Result<(), Box<dyn Error + Send + Sync>> {
    let client = rustemon::client::RustemonClient::default();
    //let all_pokemon = get_all_pokemon(&client).await;
    // let all_pokemon = match rustemon::pokemon::pokemon::get_all_entries(&client).await {
    //     Ok(list) => list,
    //     Err(e) => {
    //         println!("error getting all pokemon: {:?}", e);
    //         return Err(e.into());
    //     }
    // };

    let tmp_dir: tempfile::TempDir = match Builder::new().keep(true).tempdir() {
        Ok(d) => d,
        Err(e) => {
            println!("Error creating temporary working dir: {:?}", e);
            return Err(e.into());
        }
    };
    // creating a tempfile hold all the contents of the rdf outputs files
    // let mut combined_file = match Builder::new()
    //     .suffix(".nt")
    //     .append(true)
    //     .keep(true)
    //     .tempfile_in(tmp_dir.path())
    // {
    //     Ok(f) => f,
    //     Err(e) => {
    //         println!("Error creating temporary file: {:?}", e);
    //         return Err(e.into());
    //     }
    // };

    // creating a tempfile hold all the contents of the rdf outputs files
    // let mut combined_json_file = match Builder::new()
    //     .suffix(".json")
    //     .append(true)
    //     .keep(true)
    //     .tempfile_in(tmp_dir.path())
    // {
    //     Ok(f) => f,
    //     Err(e) => {
    //         println!("Error creating temporary file: {:?}", e);
    //         return Err(e.into());
    //     }
    // };
    // println!("json file: {}", combined_json_file.path().to_str().unwrap());
    //
    // write!(&combined_json_file, "{{ \"pokemon\": [").unwrap();

    let combined_manual_nt_file = match Builder::new()
        .suffix(".nt")
        .append(true)
        .keep(true)
        .tempfile_in(tmp_dir.path())
    {
        Ok(f) => f,
        Err(e) => {
            println!("Error creating temporary file: {:?}", e);
            return Err(e.into());
        }
    };

    let m = MultiProgress::new();
    let sty = ProgressStyle::with_template(
        "[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}",
    )
    .unwrap()
    .progress_chars("##-");
    // for res in all_pokemon.into_iter() {
    //     bar.inc(1);
    //     let pokemon = res.clone().name;
    //     let pokemon_json = match res.follow(&client).await {
    //         Ok(j) => j,
    //         Err(e) => {
    //             println!(
    //                 "error getting info for pokemon {}: {:?}",
    //                 pokemon.clone(),
    //                 e
    //             );
    //             continue;
    //         }
    //     };
    //     pokedex
    //         .pokemon_to_nt(res, pokemon_json.clone(), &client)
    //         .await?;

    //     println!("pokemon: {}", pokemon);
    //     let json_str = serde_json::to_string(&pokemon_json).unwrap();
    //     writeln!(&combined_json_file, "{},", json_str).unwrap();
    //     match json_to_rdf(&json_str, &tmp_dir, &mut combined_file).await {
    //         Ok(g) => g,
    //         Err(e) => {
    //             println!(
    //                 "error building graph for pokemon {}: {:?}",
    //                 pokemon.clone(),
    //                 e
    //             );
    //             return Err(e.into());
    //         }
    //     };
    // }
    // CHANNEL-BASED PARALLELIZATION:
    // Create an unbounded channel for sending triples from workers to writer
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();

    println!("{:?}", combined_manual_nt_file.path());

    // Spawn a dedicated writer task that consumes from the channel
    let mut output_file = combined_manual_nt_file;

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

    ability_to_nt_parallel(m.clone(), client.clone(), tx.clone()).await?;
    damage_class_to_nt_parallel(m.clone(), client.clone(), tx.clone()).await?;
    egg_group_to_nt_parallel(m.clone(), client.clone(), tx.clone()).await?;
    evolution_chain_to_nt_parallel(m.clone(), client.clone(), tx.clone()).await?;
    form_to_nt_parallel(m.clone(), client.clone(), tx.clone()).await?;
    generation_to_nt_parallel(m.clone(), client.clone(), tx.clone()).await?;
    growth_rate_to_nt_parallel(m.clone(), client.clone(), tx.clone()).await?;
    habitat_to_nt_parallel(m.clone(), client.clone(), tx.clone()).await?;
    location_to_nt_parallel(m.clone(), client.clone(), tx.clone()).await?;
    move_to_nt_parallel(m.clone(), client.clone(), tx.clone()).await?;
    move_target_to_nt_parallel(m.clone(), client.clone(), tx.clone()).await?;
    pal_park_area_to_nt_parallel(m.clone(), client.clone(), tx.clone()).await?;
    pokemon_to_nt_parallel(m.clone(), client.clone(), tx.clone()).await?;
    region_to_nt_parallel(m.clone(), client.clone(), tx.clone()).await?;
    shapes_to_nt_parallel(m.clone(), client.clone(), tx.clone()).await?;
    species_to_nt_parallel(m.clone(), client.clone(), tx.clone()).await?;
    type_to_nt_parallel(m.clone(), client.clone(), tx.clone()).await?;

    // // Wait for all worker tasks to complete
    // for handle in handles {
    //     handle.await??;
    // }

    // Drop the sender to signal the writer that no more data is coming
    drop(tx);

    // Wait for the writer to finish processing all messages
    writer_handle.await??;

    // TODO BerryFirmness
    // TODO Item
    // TODO Berry Flavor
    // TODO Berry
    // TODO ContestType
    // TODO EncounterCondition
    // TODO EncounterConditionValue
    // TODO EvolutionTrigger
    // TODO Pokedex
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
    // TODO Stat ??
    // TODO PokeathonStat
    // TODO MoveBattleStyle
    // TODO Nature
    // TODO PokemonForm
    // TODO MoveDamageClass

    // need to delete trailing comma on previous line
    // this needs to be on the same line of the last element
    // writeln!(&combined_json_file, "]}}").unwrap();
    // println!("{:?}", combined_file);

    Ok(())
}

// Helper functions to create triples
fn create_type_triple(
    subject: impl Into<oxrdf::Subject>,
    class_name: &str,
) -> Result<Triple, Box<dyn Error + Send + Sync>> {
    Ok(Triple {
        subject: subject.into(),
        predicate: vocab::rdf::TYPE.into(),
        object: NamedNode::new(format!("{}{}", POKE, class_name))?.into(),
    })
}

fn create_data_property(
    subject: impl Into<oxrdf::Subject>,
    property: &str,
    value: Literal,
) -> Result<Triple, Box<dyn Error + Send + Sync>> {
    Ok(Triple {
        subject: subject.into(),
        predicate: NamedNode::new(format!("{}{}", POKE, property))?,
        object: value.into(),
    })
}

fn create_object_property(
    subject: impl Into<oxrdf::Subject>,
    property: &str,
    object: impl Into<oxrdf::Term>,
) -> Result<Triple, Box<dyn Error + Send + Sync>> {
    Ok(Triple {
        subject: subject.into(),
        predicate: NamedNode::new(format!("{}{}", POKE, property))?,
        object: object.into(),
    })
}

async fn location_to_nt_parallel(
    bar: MultiProgress,
    client: Arc<RustemonClient>,
    tx: mpsc::UnboundedSender<String>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let all_locations = match rustemon::locations::location::get_all_entries(&client).await {
        Ok(list) => list,
        Err(e) => {
            println!("error getting all pokemon: {:?}", e);
            return Err(e.into());
        }
    };

    let pb = bar.add(ProgressBar::new(all_locations.len().try_into().unwrap()));
    let mut index = 0;
    for p in all_locations {
        pb.set_message(format!("location #{}", index + 1));
        pb.inc(1);
        index += 1;
        let mut triples: Vec<Triple> = vec![];
        let location_id = NamedNodeRef::new(p.url.as_str())?;
        let location_json = match p.follow(&client).await {
            Ok(list) => list,
            Err(e) => {
                println!("error getting all pokemon: {:?}", e);
                return Err(e.into());
            }
        };
        // Add rdf:type declaration
        triples.push(create_type_triple(location_id, "Location")?);

        triples.push(Triple {
            subject: location_id.into(),
            predicate: NamedNode::new(format!("{POKE}id"))?,
            object: Literal::new_typed_literal(location_json.id.to_string(), xsd::INTEGER).into(),
        });
        triples.push(Triple {
            subject: location_id.into(),
            predicate: NamedNode::new(format!("{POKE}name"))?,
            object: Literal::new_simple_literal(location_json.name).into(),
        });
        if let Some(region) = location_json.region {
            triples.push(Triple {
                subject: location_id.into(),
                predicate: NamedNode::new(format!("{POKE}region"))?,
                object: NamedNode::new(region.url)?.into(),
            });
        }
        for n in location_json.names {
            // TODO only english for now
            if n.language.name == "en" {
                triples.push(Triple {
                    subject: location_id.into(),
                    predicate: NamedNode::new(format!("{POKE}names"))?,
                    object: Literal::new_simple_literal(n.name).into(),
                });
            }
        }
        for gi in location_json.game_indices {
            let gi_id = BlankNode::default();
            triples.push(Triple {
                subject: location_id.into(),
                predicate: NamedNode::new(format!("{POKE}gameIndex"))?,
                object: gi_id.as_ref().into(),
            });
            triples.push(Triple {
                subject: gi_id.as_ref().into(),
                predicate: NamedNode::new(format!("{POKE}index"))?,
                object: Literal::new_typed_literal(gi.game_index.to_string(), xsd::INTEGER).into(),
            });
            triples.push(Triple {
                subject: gi_id.as_ref().into(),
                predicate: NamedNode::new(format!("{POKE}generation"))?,
                object: NamedNode::new(gi.generation.url)?.into(),
            });
        }
        for a in location_json.areas {
            triples.push(Triple {
                subject: location_id.into(),
                predicate: NamedNode::new(format!("{POKE}name"))?,
                object: NamedNode::new(a.url)?.into(),
            });
            // TODO location_area_to_nt
        }
        for t in triples {
            tx.send(format!("{t} ."))
                .map_err(|e| format!("Send error: {}", e))?
        }
    }

    Ok(())
}

async fn region_to_nt_parallel(
    bar: MultiProgress,
    client: Arc<RustemonClient>,
    tx: mpsc::UnboundedSender<String>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let all_regions = match rustemon::locations::region::get_all_entries(&client).await {
        Ok(list) => list,
        Err(e) => {
            println!("error getting all pokemon: {:?}", e);
            return Err(e.into());
        }
    };

    let pb = bar.add(ProgressBar::new(all_regions.len().try_into().unwrap()));
    let mut index = 0;
    for p in all_regions {
        pb.set_message(format!("region #{}", index + 1));
        pb.inc(1);
        index += 1;
        let mut triples: Vec<Triple> = vec![];
        let region_id = NamedNodeRef::new(p.url.as_str())?;
        let region_json = match p.follow(&client).await {
            Ok(list) => list,
            Err(e) => {
                println!("error getting all pokemon: {:?}", e);
                return Err(e.into());
            }
        };

        // Add rdf:type declaration
        triples.push(create_type_triple(region_id, "Region")?);

        triples.push(Triple {
            subject: region_id.into(),
            predicate: NamedNode::new(format!("{POKE}id"))?,
            object: Literal::new_typed_literal(region_json.id.to_string(), xsd::INTEGER).into(),
        });
        triples.push(Triple {
            subject: region_id.into(),
            predicate: NamedNode::new(format!("{POKE}name"))?,
            object: Literal::new_simple_literal(region_json.name).into(),
        });
        for n in region_json.names {
            // TODO only english for now
            if n.language.name == "en" {
                triples.push(Triple {
                    subject: region_id.into(),
                    predicate: NamedNode::new(format!("{POKE}names"))?,
                    object: Literal::new_simple_literal(n.name).into(),
                });
            }
        }
        for l in region_json.locations {
            triples.push(Triple {
                subject: region_id.into(),
                predicate: NamedNode::new(format!("{POKE}hasLocation"))?,
                object: NamedNode::new(&l.url)?.into(),
            });
        }
        for t in triples {
            tx.send(format!("{t} ."))
                .map_err(|e| format!("Send error: {}", e))?
        }
    }

    Ok(())
}

async fn generation_to_nt_parallel(
    bar: MultiProgress,
    client: Arc<RustemonClient>,
    tx: mpsc::UnboundedSender<String>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let all_generations = match rustemon::games::generation::get_all_entries(&client).await {
        Ok(list) => list,
        Err(e) => {
            println!("error getting all pokemon: {:?}", e);
            return Err(e.into());
        }
    };

    let pb = bar.add(ProgressBar::new(all_generations.len().try_into().unwrap()));
    let mut index = 0;
    for p in all_generations {
        pb.set_message(format!("generation #{}", index + 1));
        pb.inc(1);
        index += 1;
        let mut triples: Vec<Triple> = vec![];
        let gen_id = NamedNodeRef::new(p.url.as_str())?;
        let gen_json = match p.follow(&client).await {
            Ok(list) => list,
            Err(e) => {
                println!("error getting all pokemon: {:?}", e);
                return Err(e.into());
            }
        };
        // Add rdf:type declaration
        triples.push(create_type_triple(gen_id, "Generation")?);

        triples.push(Triple {
            subject: gen_id.into(),
            predicate: NamedNode::new(format!("{POKE}id"))?,
            object: Literal::new_typed_literal(gen_json.id.to_string(), xsd::INTEGER).into(),
        });
        triples.push(Triple {
            subject: gen_id.into(),
            predicate: NamedNode::new(format!("{POKE}name"))?,
            object: Literal::new_simple_literal(gen_json.name).into(),
        });
        // abilities
        for a in gen_json.abilities {
            triples.push(Triple {
                subject: gen_id.into(),
                predicate: NamedNode::new(format!("{POKE}hasNewAbiltiy"))?,
                object: NamedNode::new(a.url)?.into(),
            });
        }
        // names
        for n in gen_json.names {
            // TODO only english for now
            if n.language.name == "en" {
                triples.push(Triple {
                    subject: gen_id.into(),
                    predicate: NamedNode::new(format!("{POKE}names"))?,
                    object: Literal::new_simple_literal(n.name).into(),
                });
            }
        }
        // main_region
        triples.push(Triple {
            subject: gen_id.into(),
            predicate: NamedNode::new(format!("{POKE}region"))?,
            object: NamedNode::new(&gen_json.main_region.url)?.into(),
        });
        // moves
        for m in gen_json.moves {
            triples.push(Triple {
                subject: gen_id.into(),
                predicate: NamedNode::new(format!("{POKE}hasNewMove"))?,
                object: NamedNode::new(m.url)?.into(),
            });
        }
        // pokemon_species
        for s in gen_json.pokemon_species {
            triples.push(Triple {
                subject: gen_id.into(),
                predicate: NamedNode::new(format!("{POKE}hasNewSpecies"))?,
                object: NamedNode::new(s.url)?.into(),
            });
        }
        // types
        for t in gen_json.types {
            triples.push(Triple {
                subject: gen_id.into(),
                predicate: NamedNode::new(format!("{POKE}hasNewType"))?,
                object: NamedNode::new(t.url)?.into(),
            });
        }
        // version_groups
        for v in gen_json.version_groups {
            triples.push(Triple {
                subject: gen_id.into(),
                predicate: NamedNode::new(format!("{POKE}hasNewVersionGroup"))?,
                object: NamedNode::new(v.url)?.into(),
            });
        }
        for t in triples {
            tx.send(format!("{t} ."))
                .map_err(|e| format!("Send error: {}", e))?
        }
    }

    Ok(())
}

async fn move_target_to_nt_parallel(
    bar: MultiProgress,
    client: Arc<RustemonClient>,
    tx: mpsc::UnboundedSender<String>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let all_targets = match rustemon::moves::move_target::get_all_entries(&client).await {
        Ok(list) => list,
        Err(e) => {
            println!("error getting all pokemon: {:?}", e);
            return Err(e.into());
        }
    };

    let pb = bar.add(ProgressBar::new(all_targets.len().try_into().unwrap()));
    let mut index = 0;
    for p in all_targets {
        pb.set_message(format!("move target #{}", index + 1));
        pb.inc(1);
        index += 1;
        let mut triples: Vec<Triple> = vec![];
        let target_id = NamedNodeRef::new(p.url.as_str())?;
        let target_json = match p.follow(&client).await {
            Ok(list) => list,
            Err(e) => {
                println!("error getting all pokemon: {:?}", e);
                return Err(e.into());
            }
        };
        // Add rdf:type declaration
        triples.push(create_type_triple(target_id, "MoveTarget")?);

        triples.push(Triple {
            subject: target_id.into(),
            predicate: NamedNode::new(format!("{POKE}name"))?,
            object: Literal::new_simple_literal(p.name.clone()).into(),
        });
        for d in target_json.descriptions.clone() {
            if d.language.name == "en" {
                triples.push(Triple {
                    subject: target_id.into(),
                    predicate: NamedNode::new(format!("{POKE}description"))?,
                    object: Literal::new_simple_literal(d.description).into(),
                });
            }
        }
        for m in target_json.moves {
            triples.push(Triple {
                subject: target_id.into(),
                predicate: NamedNode::new(format!("{POKE}move"))?,
                object: NamedNode::new(m.url)?.into(),
            });
        }
        // names
        for d in target_json.names.clone() {
            // TODO only english for now
            if d.language.name == "en" {
                triples.push(Triple {
                    subject: target_id.into(),
                    predicate: NamedNode::new(format!("{POKE}names"))?,
                    object: Literal::new_simple_literal(d.name).into(),
                });
            }
        }

        for t in triples {
            tx.send(format!("{t} ."))
                .map_err(|e| format!("Send error: {}", e))?
        }
    }
    Ok(())
}
async fn damage_class_to_nt_parallel(
    bar: MultiProgress,
    client: Arc<RustemonClient>,
    tx: mpsc::UnboundedSender<String>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let all_damages = match rustemon::moves::move_damage_class::get_all_entries(&client).await {
        Ok(list) => list,
        Err(e) => {
            println!("error getting all pokemon: {:?}", e);
            return Err(e.into());
        }
    };

    let pb = bar.add(ProgressBar::new(all_damages.len().try_into().unwrap()));
    let mut index = 0;
    for p in all_damages {
        pb.set_message(format!("move damage class #{}", index + 1));
        pb.inc(1);
        index += 1;
        let mut triples: Vec<Triple> = vec![];
        let damage_id = NamedNodeRef::new(p.url.as_str())?;
        let damage_json = match p.follow(&client).await {
            Ok(list) => list,
            Err(e) => {
                println!("error getting all pokemon: {:?}", e);
                return Err(e.into());
            }
        };
        // Add rdf:type declaration
        triples.push(create_type_triple(damage_id, "MoveDamageClass")?);

        triples.push(Triple {
            subject: damage_id.into(),
            predicate: NamedNode::new(format!("{POKE}name"))?,
            object: Literal::new_simple_literal(p.name.clone()).into(),
        });

        triples.push(Triple {
            subject: damage_id.into(),
            predicate: NamedNode::new(format!("{POKE}id"))?,
            object: Literal::new_typed_literal(damage_json.id.to_string(), xsd::INTEGER).into(),
        });
        for d in damage_json.descriptions.clone() {
            // TODO only english for now
            if d.language.name == "en" {
                triples.push(Triple {
                    subject: damage_id.into(),
                    predicate: NamedNode::new(format!("{POKE}description"))?,
                    object: Literal::new_simple_literal(d.description).into(),
                });
            }
        }
        for t in triples {
            tx.send(format!("{t} ."))
                .map_err(|e| format!("Send error: {}", e))?
        }
    }

    Ok(())
}

async fn growth_rate_to_nt_parallel(
    bar: MultiProgress,
    client: Arc<RustemonClient>,
    tx: mpsc::UnboundedSender<String>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let all_rates = match rustemon::pokemon::growth_rate::get_all_entries(&client).await {
        Ok(list) => list,
        Err(e) => {
            println!("error getting all pokemon: {:?}", e);
            return Err(e.into());
        }
    };

    let pb = bar.add(ProgressBar::new(all_rates.len().try_into().unwrap()));
    let mut index = 0;
    for p in all_rates {
        pb.set_message(format!("growth rate #{}", index + 1));
        pb.inc(1);
        index += 1;
        let mut triples: Vec<Triple> = vec![];
        let growth_id = NamedNodeRef::new(p.url.as_str())?;
        let growth_json = match p.follow(&client).await {
            Ok(list) => list,
            Err(e) => {
                println!("error getting all pokemon: {:?}", e);
                return Err(e.into());
            }
        };
        // Add rdf:type declaration
        triples.push(create_type_triple(growth_id, "GrowthRate")?);

        triples.push(Triple {
            subject: growth_id.into(),
            predicate: NamedNode::new(format!("{POKE}id"))?,
            object: Literal::new_typed_literal(growth_json.id.to_string(), xsd::INTEGER).into(),
        });
        triples.push(Triple {
            subject: growth_id.into(),
            predicate: NamedNode::new(format!("{POKE}name"))?,
            object: Literal::new_simple_literal(growth_json.name).into(),
        });
        triples.push(Triple {
            subject: growth_id.into(),
            predicate: NamedNode::new(format!("{POKE}formula"))?,
            object: Literal::new_simple_literal(growth_json.formula).into(),
        });
        for d in growth_json.descriptions {
            // TODO only English for now
            if d.language.name == "en" {
                triples.push(Triple {
                    subject: growth_id.into(),
                    predicate: NamedNode::new(format!("{POKE}description"))?,
                    object: Literal::new_simple_literal(d.description).into(),
                });
            }
        }
        for l in growth_json.levels {
            let level_id = BlankNode::default();
            triples.push(Triple {
                subject: growth_id.into(),
                predicate: NamedNode::new(format!("{POKE}hasExpLevel"))?,
                object: level_id.as_ref().into(),
            });
            triples.push(Triple {
                subject: level_id.as_ref().into(),
                predicate: NamedNode::new(format!("{POKE}level"))?,
                object: Literal::new_typed_literal(l.level.to_string(), xsd::INTEGER).into(),
            });
            triples.push(Triple {
                subject: level_id.as_ref().into(),
                predicate: NamedNode::new(format!("{POKE}experience"))?,
                object: Literal::new_typed_literal(l.experience.to_string(), xsd::INTEGER).into(),
            });
        }
        // pokemon_species
        for p in growth_json.pokemon_species {
            triples.push(Triple {
                subject: growth_id.into(),
                predicate: NamedNode::new(format!("{POKE}hasSpecies"))?,
                object: NamedNode::new(p.url)?.into(),
            });
        }
        for t in triples {
            tx.send(format!("{t} ."))
                .map_err(|e| format!("Send error: {}", e))?
        }
    }

    Ok(())
}

async fn species_to_nt_parallel(
    bar: MultiProgress,
    client: Arc<RustemonClient>,
    tx: mpsc::UnboundedSender<String>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let all_species = match rustemon::pokemon::pokemon_species::get_all_entries(&client).await {
        Ok(list) => list,
        Err(e) => {
            println!("error getting all pokemon: {:?}", e);
            return Err(e.into());
        }
    };

    let pb = bar.add(ProgressBar::new(all_species.len().try_into().unwrap()));
    let mut index = 0;
    for p in all_species {
        pb.set_message(format!("species #{}", index + 1));
        pb.inc(1);
        index += 1;
        let mut triples: Vec<Triple> = vec![];
        let species_id = NamedNodeRef::new(p.url.as_str())?;
        let species_json = match p.follow(&client).await {
            Ok(list) => list,
            Err(e) => {
                println!("error getting all pokemon: {:?}", e);
                return Err(e.into());
            }
        };
        // Add rdf:type declaration
        triples.push(create_type_triple(species_id, "PokemonSpecies")?);

        triples.push(Triple {
            subject: species_id.into(),
            predicate: NamedNode::new(format!("{POKE}id"))?,
            object: Literal::new_typed_literal(species_json.id.to_string(), xsd::INTEGER).into(),
        });
        triples.push(Triple {
            subject: species_id.into(),
            predicate: NamedNode::new(format!("{POKE}name"))?,
            object: Literal::new_simple_literal(species_json.name).into(),
        });
        // TODO order
        triples.push(Triple {
            subject: species_id.into(),
            predicate: NamedNode::new(format!("{POKE}genderRate"))?,
            object: Literal::new_typed_literal(species_json.gender_rate.to_string(), xsd::INTEGER)
                .into(),
        });
        triples.push(Triple {
            subject: species_id.into(),
            predicate: NamedNode::new(format!("{POKE}captureRate"))?,
            object: Literal::new_typed_literal(species_json.capture_rate.to_string(), xsd::INTEGER)
                .into(),
        });
        if let Some(happiness) = species_json.base_hapiness {
            triples.push(Triple {
                subject: species_id.into(),
                predicate: NamedNode::new(format!("{POKE}baseHappiness"))?,
                object: Literal::new_typed_literal(happiness.to_string(), xsd::INTEGER).into(),
            });
        }

        // TODO are these bools mutually exclusive? should the triple be created only if set to true?
        triples.push(Triple {
            subject: species_id.into(),
            predicate: NamedNode::new(format!("{POKE}isBaby"))?,
            object: Literal::new_typed_literal(species_json.is_baby.to_string(), xsd::BOOLEAN)
                .into(),
        });
        triples.push(Triple {
            subject: species_id.into(),
            predicate: NamedNode::new(format!("{POKE}isLegendary"))?,
            object: Literal::new_typed_literal(species_json.is_legendary.to_string(), xsd::BOOLEAN)
                .into(),
        });
        triples.push(Triple {
            subject: species_id.into(),
            predicate: NamedNode::new(format!("{POKE}isMythical"))?,
            object: Literal::new_typed_literal(species_json.is_mythical.to_string(), xsd::BOOLEAN)
                .into(),
        });

        if let Some(counter) = species_json.hatch_counter {
            triples.push(Triple {
                subject: species_id.into(),
                predicate: NamedNode::new(format!("{POKE}hatchCounter"))?,
                object: Literal::new_typed_literal(counter.to_string(), xsd::INTEGER).into(),
            });
        }
        triples.push(Triple {
            subject: species_id.into(),
            predicate: NamedNode::new(format!("{POKE}hasGenderDifferences"))?,
            object: Literal::new_typed_literal(
                species_json.has_gender_differences.to_string(),
                xsd::BOOLEAN,
            )
            .into(),
        });
        triples.push(Triple {
            subject: species_id.into(),
            predicate: NamedNode::new(format!("{POKE}formsSwitchable"))?,
            object: Literal::new_typed_literal(
                species_json.forms_switchable.to_string(),
                xsd::BOOLEAN,
            )
            .into(),
        });

        // growth_rate
        let growth_id = NamedNodeRef::new(&species_json.growth_rate.url)?;
        triples.push(Triple {
            subject: species_id.into(),
            predicate: NamedNode::new(format!("{POKE}hasGrowthRate"))?,
            object: growth_id.into(),
        });

        // TODO pokedex_numbers
        for p in species_json.pokedex_numbers {}

        // TODO egg_groups
        for e in species_json.egg_groups {
            triples.push(Triple {
                subject: species_id.into(),
                predicate: NamedNode::new(format!("{POKE}eggGroup"))?,
                object: NamedNodeRef::new(e.url.as_str())?.into(),
            });
        }
        // TODO color
        triples.push(Triple {
            subject: species_id.into(),
            predicate: NamedNode::new(format!("{POKE}color"))?,
            object: NamedNodeRef::new(&species_json.color.url)?.into(),
        });
        // TODO shape
        if let Some(shape) = species_json.shape {
            triples.push(Triple {
                subject: species_id.into(),
                predicate: NamedNode::new(format!("{POKE}shape"))?,
                object: NamedNodeRef::new(&shape.url)?.into(),
            });
        }
        // TODO evolves_from_species
        if let Some(s) = species_json.evolves_from_species {
            triples.push(Triple {
                subject: species_id.into(),
                predicate: NamedNode::new(format!("{POKE}evolvesFrom"))?,
                object: NamedNodeRef::new(s.url.as_str())?.into(),
            });
        }
        // TODO evolution_chain
        if let Some(chain) = species_json.evolution_chain {
            triples.push(Triple {
                subject: species_id.into(),
                predicate: NamedNode::new(format!("{POKE}evolutionChain"))?,
                object: NamedNodeRef::new(chain.url.as_str())?.into(),
            });
        }
        // TODO habitat
        if let Some(habitat) = species_json.habitat {
            triples.push(Triple {
                subject: species_id.into(),
                predicate: NamedNode::new(format!("{POKE}habitat"))?,
                object: NamedNodeRef::new(habitat.url.as_str())?.into(),
            });
        }
        // generation
        triples.push(Triple {
            subject: species_id.into(),
            predicate: NamedNode::new(format!("{POKE}generation"))?,
            object: NamedNode::new(species_json.generation.url)?.into(),
        });
        // names
        for n in species_json.names {
            // TODO english only
            if n.language.name == "en" {
                triples.push(Triple {
                    subject: species_id.into(),
                    predicate: NamedNode::new(format!("{POKE}names"))?,
                    object: Literal::new_simple_literal(n.name).into(),
                });
            }
        }
        // TODO pal_park_encounters
        for enc in species_json.pal_park_encounters {
            let enc_id = BlankNode::default();
            triples.push(Triple {
                subject: species_id.into(),
                predicate: NamedNode::new(format!("{POKE}palParkEncounters"))?,
                object: enc_id.as_ref().into(),
            });
            triples.push(Triple {
                subject: enc_id.as_ref().into(),
                predicate: NamedNode::new(format!("{POKE}baseScore"))?,
                object: Literal::new_typed_literal(enc.base_score.to_string(), xsd::INTEGER).into(),
            });
            triples.push(Triple {
                subject: enc_id.as_ref().into(),
                predicate: NamedNode::new(format!("{POKE}rate"))?,
                object: Literal::new_typed_literal(enc.rate.to_string(), xsd::INTEGER).into(),
            });
            triples.push(Triple {
                subject: enc_id.as_ref().into(),
                predicate: NamedNode::new(format!("{POKE}palParkArea"))?,
                object: NamedNodeRef::new(enc.area.url.as_str())?.into(),
            });
        }

        // flavor_text_entries
        for f in species_json.flavor_text_entries {
            // TODO only english for now
            if f.language.name == "en" {
                let flavor_id = BlankNode::default();
                triples.push(Triple {
                    subject: species_id.into(),
                    predicate: NamedNode::new(format!("{POKE}flavorText"))?,
                    object: flavor_id.as_ref().into(),
                });
                triples.push(Triple {
                    subject: flavor_id.as_ref().into(),
                    predicate: NamedNode::new(format!("{POKE}text"))?,
                    object: Literal::new_simple_literal(f.flavor_text).into(),
                });
                if let Some(verison) = f.version {
                    triples.push(Triple {
                        subject: flavor_id.as_ref().into(),
                        predicate: NamedNode::new(format!("{POKE}version"))?,
                        object: NamedNode::new(verison.url)?.into(),
                    });
                }
            }
        }
        // form_descriptions
        for d in species_json.form_descriptions {
            // TODO only english for now
            if d.language.name == "en" {
                triples.push(Triple {
                    subject: species_id.into(),
                    predicate: NamedNode::new(format!("{POKE}formDescription"))?,
                    object: Literal::new_simple_literal(d.description).into(),
                });
            }
        }
        // genera
        for g in species_json.genera {
            // TODO only english for now
            if g.language.name == "en" {
                triples.push(Triple {
                    subject: species_id.into(),
                    predicate: NamedNode::new(format!("{POKE}genus"))?,
                    object: Literal::new_simple_literal(g.genus).into(),
                });
            }
        }
        // varieties
        for v in species_json.varieties {
            let v_id = NamedNodeRef::new(&v.pokemon.url)?;
            triples.push(Triple {
                subject: species_id.into(),
                predicate: NamedNode::new(format!("{POKE}hasVariety"))?,
                object: v_id.into(),
            });
            if v.is_default {
                triples.push(Triple {
                    subject: species_id.into(),
                    predicate: NamedNode::new(format!("{POKE}defaultVariety"))?,
                    object: v_id.into(),
                });
            }
        }
        for t in triples {
            tx.send(format!("{t} ."))
                .map_err(|e| format!("Send error: {}", e))?
        }
    }

    Ok(())
}

async fn evolution_chain_to_nt_parallel(
    bar: MultiProgress,
    client: Arc<RustemonClient>,
    tx: mpsc::UnboundedSender<String>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let chains = match rustemon::evolution::evolution_chain::get_all_entries(&client).await {
        Ok(list) => list,
        Err(e) => {
            println!("error getting all evolution chains: {:?}", e);
            return Err(e.into());
        }
    };

    let pb = bar.add(ProgressBar::new(chains.len().try_into().unwrap()));
    let mut index = 0;
    for p in chains {
        pb.set_message(format!("evolution chain #{}", index + 1));
        pb.inc(1);
        index += 1;
        let mut triples: Vec<Triple> = vec![];
        let chain_id = NamedNodeRef::new(p.url.as_str())?;
        let chain_json = match p.follow(&client).await {
            Ok(list) => list,
            Err(e) => {
                println!("error getting all shape: {:?}", e);
                return Err(e.into());
            }
        };
        // Add rdf:type declaration
        triples.push(create_type_triple(chain_id, "EvolutionChain")?);

        triples.push(Triple {
            subject: chain_id.into(),
            predicate: NamedNode::new(format!("{POKE}id"))?,
            object: Literal::new_typed_literal(chain_json.id.to_string(), xsd::INTEGER).into(),
        });

        if let Some(trigger_item) = chain_json.baby_trigger_item {
            triples.push(Triple {
                subject: chain_id.into(),
                predicate: NamedNode::new(format!("{POKE}triggerItem"))?,
                object: NamedNode::new(trigger_item.url.as_str())?.into(),
            });
        }

        // chain link
        triples.extend_from_slice(&chain_link_to_nt(chain_id, &chain_json.chain)?);

        // TODO evolves_to
        for evolve in &chain_json.chain.evolves_to {
            triples.extend_from_slice(&chain_link_to_nt(chain_id, &evolve)?);
        }
        for t in triples {
            tx.send(format!("{t} ."))
                .map_err(|e| format!("Send error: {}", e))?
        }
    }
    Ok(())
}

pub fn chain_link_to_nt(
    chain_id: NamedNodeRef,
    link: &ChainLink,
) -> Result<Vec<Triple>, Box<dyn Error + Send + Sync>> {
    let mut triples = vec![];
    let link_id = BlankNode::default();
    triples.push(Triple {
        subject: chain_id.into(),
        predicate: NamedNode::new(format!("{POKE}link"))?,
        object: link_id.as_ref().into(),
    });
    triples.push(Triple {
        subject: chain_id.into(),
        predicate: NamedNode::new(format!("{POKE}isBaby"))?,
        object: Literal::new_typed_literal(link.is_baby.to_string(), xsd::BOOLEAN).into(),
    });
    triples.push(Triple {
        subject: link_id.as_ref().into(),
        predicate: NamedNode::new(format!("{POKE}species"))?,
        object: NamedNode::new(link.species.url.as_str())?.into(),
    });
    for detail in &link.evolution_details {
        let detail_id = BlankNode::default();
        triples.push(Triple {
            subject: link_id.as_ref().into(),
            predicate: NamedNode::new(format!("{POKE}evolutionDetail"))?,
            object: detail_id.as_ref().into(),
        });
        if let Some(item) = &detail.item {
            triples.push(Triple {
                subject: detail_id.as_ref().into(),
                predicate: NamedNode::new(format!("{POKE}item"))?,
                object: NamedNode::new(item.url.as_str())?.into(),
            });
        }
        triples.push(Triple {
            subject: detail_id.as_ref().into(),
            predicate: NamedNode::new(format!("{POKE}trigger"))?,
            object: NamedNode::new(detail.trigger.url.as_str())?.into(),
        });
        if let Some(gender) = detail.gender {
            triples.push(Triple {
                subject: detail_id.as_ref().into(),
                predicate: NamedNode::new(format!("{POKE}gender"))?,
                object: Literal::new_typed_literal(gender.to_string(), xsd::INTEGER).into(),
            });
        }
        if let Some(item) = &detail.held_item {
            triples.push(Triple {
                subject: detail_id.as_ref().into(),
                predicate: NamedNode::new(format!("{POKE}heldItem"))?,
                object: NamedNode::new(item.url.as_str())?.into(),
            });
        }
        if let Some(known_move) = &detail.known_move {
            triples.push(Triple {
                subject: detail_id.as_ref().into(),
                predicate: NamedNode::new(format!("{POKE}knownMove"))?,
                object: NamedNode::new(known_move.url.as_str())?.into(),
            });
        }
        if let Some(move_type) = &detail.known_move_type {
            triples.push(Triple {
                subject: detail_id.as_ref().into(),
                predicate: NamedNode::new(format!("{POKE}knownMoveType"))?,
                object: NamedNode::new(move_type.url.as_str())?.into(),
            });
        }
        if let Some(loc) = &detail.location {
            triples.push(Triple {
                subject: detail_id.as_ref().into(),
                predicate: NamedNode::new(format!("{POKE}item"))?,
                object: NamedNode::new(loc.url.as_str())?.into(),
            });
        }
        if let Some(lvl) = detail.min_level {
            triples.push(Triple {
                subject: detail_id.as_ref().into(),
                predicate: NamedNode::new(format!("{POKE}minLevel"))?,
                object: Literal::new_typed_literal(lvl.to_string(), xsd::INTEGER).into(),
            });
        }
        if let Some(happy) = detail.min_happiness {
            triples.push(Triple {
                subject: detail_id.as_ref().into(),
                predicate: NamedNode::new(format!("{POKE}minHappiness"))?,
                object: Literal::new_typed_literal(happy.to_string(), xsd::INTEGER).into(),
            });
        }
        if let Some(beauty) = detail.min_beauty {
            triples.push(Triple {
                subject: detail_id.as_ref().into(),
                predicate: NamedNode::new(format!("{POKE}minBeauty"))?,
                object: Literal::new_typed_literal(beauty.to_string(), xsd::INTEGER).into(),
            });
        }
        if let Some(affection) = detail.min_affection {
            triples.push(Triple {
                subject: detail_id.as_ref().into(),
                predicate: NamedNode::new(format!("{POKE}minAffections"))?,
                object: Literal::new_typed_literal(affection.to_string(), xsd::INTEGER).into(),
            });
        }
        if detail.needs_overworld_rain {
            triples.push(Triple {
                subject: detail_id.as_ref().into(),
                predicate: NamedNode::new(format!("{POKE}needsOverworldRain"))?,
                object: Literal::new_typed_literal(
                    detail.needs_overworld_rain.to_string(),
                    xsd::BOOLEAN,
                )
                .into(),
            });
        }
        if let Some(spec) = &detail.party_species {
            triples.push(Triple {
                subject: detail_id.as_ref().into(),
                predicate: NamedNode::new(format!("{POKE}partySpecies"))?,
                object: NamedNode::new(spec.url.as_str())?.into(),
            });
        }
        if let Some(party_type) = &detail.party_type {
            triples.push(Triple {
                subject: detail_id.as_ref().into(),
                predicate: NamedNode::new(format!("{POKE}partyType"))?,
                object: NamedNode::new(party_type.url.as_str())?.into(),
            });
        }
        if let Some(stats) = detail.relative_physical_stats {
            triples.push(Triple {
                subject: detail_id.as_ref().into(),
                predicate: NamedNode::new(format!("{POKE}relativePhysicalStats"))?,
                object: Literal::new_typed_literal(stats.to_string(), xsd::INTEGER).into(),
            });
        }
        triples.push(Triple {
            subject: detail_id.as_ref().into(),
            predicate: NamedNode::new(format!("{POKE}timeOfDay"))?,
            object: Literal::new_simple_literal(&detail.time_of_day).into(),
        });
        if let Some(spec) = &detail.trade_species {
            triples.push(Triple {
                subject: detail_id.as_ref().into(),
                predicate: NamedNode::new(format!("{POKE}tradeSpecies"))?,
                object: NamedNode::new(spec.url.as_str())?.into(),
            });
        }
        if detail.turn_upside_down {
            triples.push(Triple {
                subject: detail_id.as_ref().into(),
                predicate: NamedNode::new(format!("{POKE}turnUpsideDown"))?,
                object: Literal::new_typed_literal(
                    detail.turn_upside_down.to_string(),
                    xsd::BOOLEAN,
                )
                .into(),
            })
        }
    }
    Ok(triples)
}

async fn pal_park_area_to_nt_parallel(
    bar: MultiProgress,
    client: Arc<RustemonClient>,
    tx: mpsc::UnboundedSender<String>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let areas = match rustemon::locations::pal_park_area::get_all_entries(&client).await {
        Ok(list) => list,
        Err(e) => {
            println!("error getting all pal park areas: {:?}", e);
            return Err(e.into());
        }
    };

    let pb = bar.add(ProgressBar::new(areas.len().try_into().unwrap()));
    let mut index = 0;
    for p in areas {
        pb.set_message(format!("pal park area #{}", index + 1));
        pb.inc(1);
        index += 1;
        let mut triples: Vec<Triple> = vec![];
        let area_id = NamedNodeRef::new(p.url.as_str())?;
        let area_json = match p.follow(&client).await {
            Ok(list) => list,
            Err(e) => {
                println!("error getting pal park area: {:?}", e);
                return Err(e.into());
            }
        };
        // Add rdf:type declaration
        triples.push(create_type_triple(area_id, "PalParkArea")?);

        triples.push(Triple {
            subject: area_id.into(),
            predicate: NamedNode::new(format!("{POKE}id"))?,
            object: Literal::new_typed_literal(area_json.id.to_string(), xsd::INTEGER).into(),
        });
        triples.push(Triple {
            subject: area_id.into(),
            predicate: NamedNode::new(format!("{POKE}name"))?,
            object: Literal::new_simple_literal(area_json.name).into(),
        });

        for name in area_json.names {
            // TODO only english for now
            if name.language.name == "en" {
                triples.push(Triple {
                    subject: area_id.into(),
                    predicate: NamedNode::new(format!("{POKE}names"))?,
                    object: Literal::new_simple_literal(name.name).into(),
                });
            }
        }

        for enc in area_json.pokemon_encounters {
            let enc_id = BlankNode::default();
            triples.push(Triple {
                subject: area_id.into(),
                predicate: NamedNode::new(format!("{POKE}pokemonEncounters"))?,
                object: enc_id.as_ref().into(),
            });
            triples.push(Triple {
                subject: enc_id.as_ref().into(),
                predicate: NamedNode::new(format!("{POKE}baseScore"))?,
                object: Literal::new_typed_literal(enc.base_score.to_string(), xsd::INTEGER).into(),
            });
            triples.push(Triple {
                subject: enc_id.as_ref().into(),
                predicate: NamedNode::new(format!("{POKE}rate"))?,
                object: Literal::new_typed_literal(enc.rate.to_string(), xsd::INTEGER).into(),
            });
            triples.push(Triple {
                subject: enc_id.as_ref().into(),
                predicate: NamedNode::new(format!("{POKE}pokemonSpecies"))?,
                object: NamedNode::new(enc.pokemon_species.url.as_str())?.into(),
            });
        }

        for t in triples {
            tx.send(format!("{t} ."))
                .map_err(|e| format!("Send error: {}", e))?
        }
    }

    Ok(())
}

async fn habitat_to_nt_parallel(
    bar: MultiProgress,
    client: Arc<RustemonClient>,
    tx: mpsc::UnboundedSender<String>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let habitats = match rustemon::pokemon::pokemon_habitat::get_all_entries(&client).await {
        Ok(list) => list,
        Err(e) => {
            println!("error getting all shapes: {:?}", e);
            return Err(e.into());
        }
    };

    let pb = bar.add(ProgressBar::new(habitats.len().try_into().unwrap()));
    let mut index = 0;
    for p in habitats {
        pb.set_message(format!("habitat #{}", index + 1));
        pb.inc(1);
        index += 1;
        let mut triples: Vec<Triple> = vec![];
        let habitat_id = NamedNodeRef::new(p.url.as_str())?;
        let habitat_json = match p.follow(&client).await {
            Ok(list) => list,
            Err(e) => {
                println!("error getting all habitats: {:?}", e);
                return Err(e.into());
            }
        };
        // Add rdf:type declaration
        triples.push(create_type_triple(habitat_id, "Habitat")?);

        triples.push(Triple {
            subject: habitat_id.into(),
            predicate: NamedNode::new(format!("{POKE}id"))?,
            object: Literal::new_typed_literal(habitat_json.id.to_string(), xsd::INTEGER).into(),
        });
        triples.push(Triple {
            subject: habitat_id.into(),
            predicate: NamedNode::new(format!("{POKE}name"))?,
            object: Literal::new_simple_literal(habitat_json.name).into(),
        });

        for name in habitat_json.names {
            // TODO only english for now
            if name.language.name == "en" {
                triples.push(Triple {
                    subject: habitat_id.into(),
                    predicate: NamedNode::new(format!("{POKE}names"))?,
                    object: Literal::new_simple_literal(name.name).into(),
                });
            }
        }

        // TODO pokemon species: skip since same query can be acheived by querying '?species pdb:habitat id'

        for t in triples {
            tx.send(format!("{t} ."))
                .map_err(|e| format!("Send error: {}", e))?
        }
    }

    Ok(())
}

async fn shapes_to_nt_parallel(
    bar: MultiProgress,
    client: Arc<RustemonClient>,
    tx: mpsc::UnboundedSender<String>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let shapes = match rustemon::pokemon::pokemon_shape::get_all_entries(&client).await {
        Ok(list) => list,
        Err(e) => {
            println!("error getting all shapes: {:?}", e);
            return Err(e.into());
        }
    };

    let pb = bar.add(ProgressBar::new(shapes.len().try_into().unwrap()));
    let mut index = 0;
    for p in shapes {
        pb.set_message(format!("shape #{}", index + 1));
        pb.inc(1);
        index += 1;
        let mut triples: Vec<Triple> = vec![];
        let shape_id = NamedNodeRef::new(p.url.as_str())?;
        let shape_json = match p.follow(&client).await {
            Ok(list) => list,
            Err(e) => {
                println!("error getting all shape: {:?}", e);
                return Err(e.into());
            }
        };
        // Add rdf:type declaration
        triples.push(create_type_triple(shape_id, "PokemonShape")?);

        triples.push(Triple {
            subject: shape_id.into(),
            predicate: NamedNode::new(format!("{POKE}id"))?,
            object: Literal::new_typed_literal(shape_json.id.to_string(), xsd::INTEGER).into(),
        });
        triples.push(Triple {
            subject: shape_id.into(),
            predicate: NamedNode::new(format!("{POKE}name"))?,
            object: Literal::new_simple_literal(shape_json.name).into(),
        });

        for name in shape_json.awesome_names {
            // TODO only english for now
            if name.language.name == "en" {
                triples.push(Triple {
                    subject: shape_id.into(),
                    predicate: NamedNode::new(format!("{POKE}awsomeNames"))?,
                    object: Literal::new_simple_literal(name.awesome_name).into(),
                });
            }
        }

        for name in shape_json.names {
            // TODO only english for now
            if name.language.name == "en" {
                triples.push(Triple {
                    subject: shape_id.into(),
                    predicate: NamedNode::new(format!("{POKE}names"))?,
                    object: Literal::new_simple_literal(name.name).into(),
                });
            }
        }

        // TODO pokemon species: skip since same query can be acheived by querying '?species pdb:shape id'

        for t in triples {
            tx.send(format!("{t} ."))
                .map_err(|e| format!("Send error: {}", e))?
        }
    }
    Ok(())
}

async fn egg_group_to_nt_parallel(
    bar: MultiProgress,
    client: Arc<RustemonClient>,
    tx: mpsc::UnboundedSender<String>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let egg_groups = match rustemon::pokemon::egg_group::get_all_entries(&client).await {
        Ok(list) => list,
        Err(e) => {
            println!("error getting all egg groups: {:?}", e);
            return Err(e.into());
        }
    };

    let pb = bar.add(ProgressBar::new(egg_groups.len().try_into().unwrap()));
    let mut index = 0;
    for p in egg_groups {
        pb.set_message(format!("egg group #{}", index + 1));
        pb.inc(1);
        index += 1;
        let mut triples: Vec<Triple> = vec![];
        let group_id = NamedNodeRef::new(p.url.as_str())?;
        let group_json = match p.follow(&client).await {
            Ok(list) => list,
            Err(e) => {
                println!("error getting all egg groups: {:?}", e);
                return Err(e.into());
            }
        };
        // Add rdf:type declaration
        triples.push(create_type_triple(group_id, "EggGroup")?);

        triples.push(Triple {
            subject: group_id.into(),
            predicate: NamedNode::new(format!("{POKE}id"))?,
            object: Literal::new_typed_literal(group_json.id.to_string(), xsd::INTEGER).into(),
        });
        triples.push(Triple {
            subject: group_id.into(),
            predicate: NamedNode::new(format!("{POKE}name"))?,
            object: Literal::new_simple_literal(group_json.name).into(),
        });
        for name in group_json.names {
            // TODO only english for now
            if name.language.name == "en" {
                triples.push(Triple {
                    subject: group_id.into(),
                    predicate: NamedNode::new(format!("{POKE}names"))?,
                    object: Literal::new_simple_literal(name.name).into(),
                });
            }
        }
        // TODO pokemon species: skip since same query can be acheived by querying '?species pdb:egg_group id'

        for t in triples {
            tx.send(format!("{t} ."))
                .map_err(|e| format!("Send error: {}", e))?
        }
    }
    Ok(())
}

async fn form_to_nt_parallel(
    bar: MultiProgress,
    client: Arc<RustemonClient>,
    tx: mpsc::UnboundedSender<String>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let all_forms = match rustemon::pokemon::pokemon_form::get_all_entries(&client).await {
        Ok(list) => list,
        Err(e) => {
            println!("error getting all pokemon: {:?}", e);
            return Err(e.into());
        }
    };

    let pb = bar.add(ProgressBar::new(all_forms.len().try_into().unwrap()));
    let mut index = 0;
    for p in all_forms {
        pb.set_message(format!("form #{}", index + 1));
        pb.inc(1);
        index += 1;
        let mut triples: Vec<Triple> = vec![];
        let form_id = NamedNodeRef::new(p.url.as_str())?;
        let form_json = match p.follow(&client).await {
            Ok(list) => list,
            Err(e) => {
                println!("error getting all pokemon: {:?}", e);
                return Err(e.into());
            }
        };
        // Add rdf:type declaration
        triples.push(create_type_triple(form_id, "PokemonForm")?);

        triples.push(Triple {
            subject: form_id.into(),
            predicate: NamedNode::new(format!("{POKE}id"))?,
            object: Literal::new_typed_literal(form_json.id.to_string(), xsd::INTEGER).into(),
        });
        triples.push(Triple {
            subject: form_id.into(),
            predicate: NamedNode::new(format!("{POKE}name"))?,
            object: Literal::new_simple_literal(form_json.name).into(),
        });
        // TODO order
        // TODO form_order
        triples.push(Triple {
            subject: form_id.into(),
            predicate: NamedNode::new(format!("{POKE}formName"))?,
            object: Literal::new_simple_literal(form_json.form_name).into(),
        });
        triples.push(Triple {
            subject: form_id.into(),
            predicate: NamedNode::new(format!("{POKE}isBattleOnly"))?,
            object: Literal::new_typed_literal(form_json.is_battle_only.to_string(), xsd::BOOLEAN)
                .into(),
        });
        triples.push(Triple {
            subject: form_id.into(),
            predicate: NamedNode::new(format!("{POKE}isDefault"))?,
            object: Literal::new_typed_literal(form_json.is_default.to_string(), xsd::BOOLEAN)
                .into(),
        });
        triples.push(Triple {
            subject: form_id.into(),
            predicate: NamedNode::new(format!("{POKE}isMega"))?,
            object: Literal::new_typed_literal(form_json.is_mega.to_string(), xsd::BOOLEAN).into(),
        });

        // pokemon
        triples.push(Triple {
            subject: form_id.into(),
            predicate: NamedNode::new(format!("{POKE}pokemon"))?,
            object: NamedNode::new(form_json.pokemon.url)?.into(),
        });

        for t in form_json.types {
            triples.push(Triple {
                subject: form_id.into(),
                predicate: NamedNode::new(format!("{POKE}hasType"))?,
                object: NamedNode::new(t.type_.url)?.into(),
            });
            // type information is already collected at the top level for pokemon, no need to duplicate the get logic here too
        }

        // TODO sprites
        // version_group
        triples.push(Triple {
            subject: form_id.into(),
            predicate: NamedNode::new(format!("{POKE}versionGroup"))?,
            object: NamedNode::new(form_json.version_group.url)?.into(),
        });
        // TODO version_group_to_nt
        // names
        for n in form_json.names {
            // TODO only english for now
            if n.language.name == "en" {
                triples.push(Triple {
                    subject: form_id.into(),
                    predicate: NamedNode::new(format!("{POKE}names"))?,
                    object: Literal::new_simple_literal(n.name).into(),
                });
            }
        }
        // form_names
        for f in form_json.form_names {
            // TODO only english for now
            if f.language.name == "en" {
                triples.push(Triple {
                    subject: form_id.into(),
                    predicate: NamedNode::new(format!("{POKE}formNames"))?,
                    object: Literal::new_simple_literal(f.name).into(),
                });
            }
        }
        for t in triples {
            tx.send(format!("{t} ."))
                .map_err(|e| format!("Send error: {}", e))?
        }
    }

    Ok(())
}

async fn type_to_nt_parallel(
    bar: MultiProgress,
    client: Arc<RustemonClient>,
    tx: mpsc::UnboundedSender<String>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let all_types = match rustemon::pokemon::type_::get_all_entries(&client).await {
        Ok(list) => list,
        Err(e) => {
            println!("error getting all types: {:?}", e);
            return Err(e.into());
        }
    };
    let pb = bar.add(ProgressBar::new(all_types.len().try_into().unwrap()));
    let mut index = 0;
    for t in all_types {
        //if !self.types.contains(&t.type_.url) {
        pb.set_message(format!("type #{}", index + 1));
        pb.inc(1);
        index += 1;
        let mut triples = vec![];
        //self.types.insert(t.url.clone());
        let type_id = NamedNodeRef::new(&t.url)?;
        let type_json = match t.follow(&client).await {
            Ok(v) => v,
            Err(e) => {
                eprintln!("Error getting type info for {}: {e}", &t.url);
                return Err(e.into());
            }
        };
        // Add rdf:type declaration
        triples.push(create_type_triple(type_id, "PokemonType")?);

        triples.push(Triple {
            subject: type_id.into(),
            predicate: NamedNode::new(format!("{POKE}name"))?,
            object: Literal::new_simple_literal(type_json.name).into(),
        });
        triples.push(Triple {
            subject: type_id.into(),
            predicate: NamedNode::new(format!("{POKE}id"))?,
            object: Literal::new_typed_literal(type_json.id.to_string(), xsd::INTEGER).into(),
        });
        for m in type_json.damage_relations.double_damage_from.clone() {
            triples.push(Triple {
                subject: type_id.into(),
                predicate: NamedNode::new(format!("{POKE}doubleDamageFrom"))?,
                object: Literal::new_simple_literal(m.url).into(),
            });
        }
        for m in type_json.damage_relations.double_damage_to.clone() {
            triples.push(Triple {
                subject: type_id.into(),
                predicate: NamedNode::new(format!("{POKE}doubleDamageTo"))?,
                object: Literal::new_simple_literal(m.url).into(),
            });
        }
        for m in type_json.damage_relations.half_damage_from.clone() {
            triples.push(Triple {
                subject: type_id.into(),
                predicate: NamedNode::new(format!("{POKE}halfDamageFrom"))?,
                object: Literal::new_simple_literal(m.url).into(),
            });
        }
        for m in type_json.damage_relations.half_damage_to.clone() {
            triples.push(Triple {
                subject: type_id.into(),
                predicate: NamedNode::new(format!("{POKE}halfDamageTo"))?,
                object: Literal::new_simple_literal(m.url).into(),
            });
        }
        for m in type_json.damage_relations.no_damage_from.clone() {
            triples.push(Triple {
                subject: type_id.into(),
                predicate: NamedNode::new(format!("{POKE}noDamageFrom"))?,
                object: Literal::new_simple_literal(m.url).into(),
            });
        }
        for m in type_json.damage_relations.no_damage_to.clone() {
            triples.push(Triple {
                subject: type_id.into(),
                predicate: NamedNode::new(format!("{POKE}noDamageTo"))?,
                object: Literal::new_simple_literal(m.url).into(),
            });
        }
        // TODO past_damage_relations
        for gi in type_json.game_indices {
            let gi_id = BlankNode::default();
            triples.push(Triple {
                subject: type_id.into(),
                predicate: NamedNode::new(format!("{POKE}gameIndex"))?,
                object: gi_id.as_ref().into(),
            });
            triples.push(Triple {
                subject: gi_id.as_ref().into(),
                predicate: NamedNode::new(format!("{POKE}index"))?,
                object: Literal::new_typed_literal(gi.game_index.to_string(), xsd::INTEGER).into(),
            });
            triples.push(Triple {
                subject: gi_id.as_ref().into(),
                predicate: NamedNode::new(format!("{POKE}generation"))?,
                object: NamedNode::new(gi.generation.url)?.into(),
            });
        }
        triples.push(Triple {
            subject: type_id.into(),
            predicate: NamedNode::new(format!("{POKE}generation"))?,
            object: NamedNode::new(type_json.generation.url)?.into(),
        });
        for n in type_json.names {
            // TODO only english for now
            if n.language.name == "en" {
                triples.push(Triple {
                    subject: type_id.into(),
                    predicate: NamedNode::new(format!("{POKE}names"))?,
                    object: Literal::new_simple_literal(n.name).into(),
                });
            }
        }
        if let Some(damage) = type_json.move_damage_class {
            triples.push(Triple {
                subject: type_id.into(),
                predicate: NamedNode::new(format!("{POKE}damageClass"))?,
                object: NamedNode::new(&damage.url)?.into(),
            });
        }
        for p in type_json.pokemon {
            triples.push(Triple {
                subject: type_id.into(),
                predicate: NamedNode::new(format!("{POKE}pokemon"))?,
                object: NamedNode::new(p.pokemon.url)?.into(),
            });
        }
        for m in type_json.moves {
            triples.push(Triple {
                subject: type_id.into(),
                predicate: NamedNode::new(format!("{POKE}move"))?,
                object: NamedNode::new(m.url)?.into(),
            });
        }
        for t in triples {
            tx.send(format!("{t} ."))
                .map_err(|e| format!("Send error: {}", e))?
        }
    }
    Ok(())
}

async fn move_to_nt_parallel(
    bar: MultiProgress,
    client: Arc<RustemonClient>,
    tx: mpsc::UnboundedSender<String>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let all_moves = match rustemon::moves::move_::get_all_entries(&client).await {
        Ok(list) => list,
        Err(e) => {
            println!("error getting all pokemon: {:?}", e);
            return Err(e.into());
        }
    };
    let pb = bar.add(ProgressBar::new(all_moves.len().try_into().unwrap()));
    let mut index = 0;
    for m in all_moves {
        pb.set_message(format!("move #{}", index + 1));
        pb.inc(1);
        index += 1;
        //if !self.moves.contains(&m.move_.url) {
        let mut triples = vec![];
        //self.moves.insert(m.move_.url.clone());
        let move_id = NamedNodeRef::new(&m.url)?;
        // Add rdf:type declaration
        triples.push(create_type_triple(move_id, "Move")?);

        triples.push(Triple {
            subject: move_id.into(),
            predicate: NamedNode::new(format!("{POKE}name"))?,
            object: Literal::new_simple_literal(m.name.clone()).into(),
        });
        let move_json = match m.follow(&client).await {
            Ok(v) => v,
            Err(e) => {
                eprintln!("Error getting type info for {}: {e}", &m.url);
                return Err(e.into());
            }
        };
        triples.push(Triple {
            subject: move_id.into(),
            predicate: NamedNode::new(format!("{POKE}id"))?,
            object: Literal::new_typed_literal(move_json.id.to_string(), xsd::INTEGER).into(),
        });
        if move_json.accuracy.is_some() {
            triples.push(Triple {
                subject: move_id.into(),
                predicate: NamedNode::new(format!("{POKE}accuracy"))?,
                object: Literal::new_typed_literal(
                    move_json.accuracy.unwrap().to_string(),
                    xsd::INTEGER,
                )
                .into(),
            });
        }
        if move_json.effect_chance.is_some() {
            triples.push(Triple {
                subject: move_id.into(),
                predicate: NamedNode::new(format!("{POKE}effectChance"))?,
                object: Literal::new_typed_literal(
                    move_json.effect_chance.unwrap().to_string(),
                    xsd::INTEGER,
                )
                .into(),
            });
        }
        if move_json.pp.is_some() {
            triples.push(Triple {
                subject: move_id.into(),
                predicate: NamedNode::new(format!("{POKE}pp"))?,
                object: Literal::new_typed_literal(move_json.pp.unwrap().to_string(), xsd::INTEGER)
                    .into(),
            });
        }
        triples.push(Triple {
            subject: move_id.into(),
            predicate: NamedNode::new(format!("{POKE}priority"))?,
            object: Literal::new_typed_literal(move_json.priority.to_string(), xsd::INTEGER).into(),
        });
        if move_json.power.is_some() {
            triples.push(Triple {
                subject: move_id.into(),
                predicate: NamedNode::new(format!("{POKE}power"))?,
                object: Literal::new_typed_literal(
                    move_json.power.unwrap().to_string(),
                    xsd::INTEGER,
                )
                .into(),
            });
        }
        // TODO contest_combos
        // TODO contest_type
        // TODO contest_effect
        // damage_class
        triples.push(Triple {
            subject: move_id.into(),
            predicate: NamedNode::new(format!("{POKE}damageClass"))?,
            object: NamedNode::new(&move_json.damage_class.url)?.into(),
        });

        for effect in move_json.effect_entries.clone() {
            // TODO only english for now
            if effect.language.name == "en" {
                triples.push(Triple {
                    subject: move_id.into(),
                    predicate: NamedNode::new(format!("{POKE}effect"))?,
                    object: Literal::new_simple_literal(effect.effect).into(),
                });
                triples.push(Triple {
                    subject: move_id.into(),
                    predicate: NamedNode::new(format!("{POKE}shortEffect"))?,
                    object: Literal::new_simple_literal(effect.short_effect).into(),
                });
            }
        }
        for effect in move_json.flavor_text_entries.clone() {
            // TODO only english for now
            if effect.language.name == "en" {
                let flavor_id = BlankNode::default();
                triples.push(Triple {
                    subject: move_id.into(),
                    predicate: NamedNode::new(format!("{POKE}flavorText"))?,
                    object: flavor_id.as_ref().into(),
                });
                triples.push(Triple {
                    subject: flavor_id.as_ref().into(),
                    predicate: NamedNode::new(format!("{POKE}text"))?,
                    object: Literal::new_simple_literal(effect.flavor_text).into(),
                });
                triples.push(Triple {
                    subject: flavor_id.as_ref().into(),
                    predicate: NamedNode::new(format!("{POKE}versionGroup"))?,
                    object: NamedNode::new(effect.version_group.url)?.into(),
                });
                // TODO version_group_to_nt
            }
        }
        // learned_by_pokemon
        for p in move_json.learned_by_pokemon {
            triples.push(Triple {
                subject: move_id.into(),
                predicate: NamedNode::new(format!("{POKE}learnedBy"))?,
                object: NamedNode::new(p.url)?.into(),
            });
        }
        // generation
        triples.push(Triple {
            subject: move_id.into(),
            predicate: NamedNode::new(format!("{POKE}generation"))?,
            object: NamedNode::new(&move_json.generation.url)?.into(),
        });

        // TODO machines: this is going to be generation specific, skip until generation is implemented everywhere
        if let Some(meta) = move_json.meta {
            // TODO anything else important in MoveAilment?
            triples.push(Triple {
                subject: move_id.into(),
                predicate: NamedNode::new(format!("{POKE}ailment"))?,
                object: Literal::new_simple_literal(meta.ailment.name).into(),
            });
            // TODO anything else important in MoveCategory
            triples.push(Triple {
                subject: move_id.into(),
                predicate: NamedNode::new(format!("{POKE}category"))?,
                object: Literal::new_simple_literal(meta.category.name).into(),
            });

            triples.push(Triple {
                subject: move_id.into(),
                predicate: NamedNode::new(format!("{POKE}ailmentChance"))?,
                object: Literal::new_typed_literal(meta.ailment_chance.to_string(), xsd::INTEGER)
                    .into(),
            });
            triples.push(Triple {
                subject: move_id.into(),
                predicate: NamedNode::new(format!("{POKE}drain"))?,
                object: Literal::new_typed_literal(meta.drain.to_string(), xsd::INTEGER).into(),
            });
            triples.push(Triple {
                subject: move_id.into(),
                predicate: NamedNode::new(format!("{POKE}healing"))?,
                object: Literal::new_typed_literal(meta.healing.to_string(), xsd::INTEGER).into(),
            });
            triples.push(Triple {
                subject: move_id.into(),
                predicate: NamedNode::new(format!("{POKE}critRate"))?,
                object: Literal::new_typed_literal(meta.crit_rate.to_string(), xsd::INTEGER).into(),
            });
            triples.push(Triple {
                subject: move_id.into(),
                predicate: NamedNode::new(format!("{POKE}flinchChance"))?,
                object: Literal::new_simple_literal(meta.flinch_chance.to_string()).into(),
            });
            triples.push(Triple {
                subject: move_id.into(),
                predicate: NamedNode::new(format!("{POKE}statChance"))?,
                object: Literal::new_typed_literal(meta.stat_chance.to_string(), xsd::INTEGER)
                    .into(),
            });
            if let Some(hits) = meta.min_hits {
                triples.push(Triple {
                    subject: move_id.into(),
                    predicate: NamedNode::new(format!("{POKE}minHits"))?,
                    object: Literal::new_typed_literal(hits.to_string(), xsd::INTEGER).into(),
                });
            }
            if let Some(hits) = meta.max_hits {
                triples.push(Triple {
                    subject: move_id.into(),
                    predicate: NamedNode::new(format!("{POKE}maxHits"))?,
                    object: Literal::new_typed_literal(hits.to_string(), xsd::INTEGER).into(),
                });
            }
            if let Some(turns) = meta.min_turns {
                triples.push(Triple {
                    subject: move_id.into(),
                    predicate: NamedNode::new(format!("{POKE}minTurns"))?,
                    object: Literal::new_typed_literal(turns.to_string(), xsd::INTEGER).into(),
                });
            }
            if let Some(turns) = meta.max_turns {
                triples.push(Triple {
                    subject: move_id.into(),
                    predicate: NamedNode::new(format!("{POKE}maxTurns"))?,
                    object: Literal::new_typed_literal(turns.to_string(), xsd::INTEGER).into(),
                });
            }
        }
        // names
        for n in move_json.names {
            if n.language.name == "en" {
                triples.push(Triple {
                    subject: move_id.into(),
                    predicate: NamedNode::new(format!("{POKE}names"))?,
                    object: Literal::new_simple_literal(n.name).into(),
                });
            }
        }
        // TODO past_values
        for stat in move_json.stat_changes.clone() {
            let stat_change_id = NamedNode::new(stat.stat.url)?;
            triples.push(Triple {
                subject: move_id.into(),
                predicate: NamedNode::new(format!("{POKE}statChanges"))?,
                object: stat_change_id.as_ref().into(),
            });
            triples.push(Triple {
                subject: stat_change_id.as_ref().into(),
                predicate: NamedNode::new(format!("{POKE}name"))?,
                object: Literal::new_simple_literal(stat.stat.name).into(),
            });
            triples.push(Triple {
                subject: stat_change_id.as_ref().into(),
                predicate: NamedNode::new(format!("{POKE}change"))?,
                object: Literal::new_typed_literal(stat.change.to_string(), xsd::INTEGER).into(),
            });
            // TODO get stat URL?
        }
        // TODO super_contest_effect

        // move_target
        triples.push(Triple {
            subject: move_id.into(),
            predicate: NamedNode::new(format!("{POKE}target"))?,
            object: NamedNode::new(move_json.target.url.clone())?.into(),
        });

        triples.push(Triple {
            subject: move_id.into(),
            predicate: NamedNode::new(format!("{POKE}hasType"))?,
            object: NamedNode::new(move_json.type_.url)?.into(),
        });

        for t in triples {
            tx.send(format!("{t} ."))
                .map_err(|e| format!("Send error: {}", e))?
        }
    }
    Ok(())
}

async fn ability_to_nt_parallel(
    bar: MultiProgress,
    client: Arc<RustemonClient>,
    tx: mpsc::UnboundedSender<String>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let all_abilities = match rustemon::pokemon::ability::get_all_entries(&client).await {
        Ok(list) => list,
        Err(e) => {
            println!("error getting all pokemon: {:?}", e);
            return Err(e.into());
        }
    };

    let pb = bar.add(ProgressBar::new(all_abilities.len().try_into().unwrap()));
    let mut index = 0;
    for p in all_abilities {
        pb.set_message(format!("ability #{}", index + 1));
        pb.inc(1);
        index += 1;
        let mut triples: Vec<Triple> = vec![];
        let ability_id = NamedNodeRef::new(p.url.as_str())?;
        let ability_json = match p.follow(&client).await {
            Ok(list) => list,
            Err(e) => {
                println!("error getting all pokemon: {:?}", e);
                return Err(e.into());
            }
        };
        // Add rdf:type declaration
        triples.push(create_type_triple(ability_id, "Ability")?);

        triples.push(Triple {
            subject: ability_id.into(),
            predicate: NamedNode::new(format!("{POKE}name"))?,
            object: Literal::new_simple_literal(ability_json.name.clone()).into(),
        });
        triples.push(Triple {
            subject: ability_id.into(),
            predicate: NamedNode::new(format!("{POKE}id"))?,
            object: Literal::new_typed_literal(ability_json.id.to_string(), xsd::INTEGER).into(),
        });

        // TODO is_main_series

        // generation
        let gen_id = NamedNodeRef::new(&ability_json.generation.url)?;
        triples.push(Triple {
            subject: ability_id.into(),
            predicate: NamedNode::new(format!("{POKE}generation"))?,
            object: gen_id.into(),
        });

        for v in ability_json.effect_entries {
            // TODO only do english for now
            if v.language.name == "en" {
                triples.push(Triple {
                    subject: ability_id.into(),
                    predicate: NamedNode::new(format!("{POKE}effect"))?,
                    object: Literal::new_simple_literal(v.effect).into(),
                });
                triples.push(Triple {
                    subject: ability_id.into(),
                    predicate: NamedNode::new(format!("{POKE}shortEffect"))?,
                    object: Literal::new_simple_literal(v.short_effect).into(),
                });
            }
        }

        // TODO effect_changes

        for v in ability_json.flavor_text_entries {
            // TODO only do english for now
            if v.language.name == "en" {
                triples.push(Triple {
                    subject: ability_id.into(),
                    predicate: NamedNode::new(format!("{POKE}flavorText"))?,
                    object: Literal::new_simple_literal(v.flavor_text).into(),
                });
            }
        }

        // TODO pokemon: skip since same query can be acheived by querying '?pokemon pdb:hasAbility id'

        for t in triples {
            tx.send(format!("{t} ."))
                .map_err(|e| format!("Send error: {}", e))?
        }
    }

    Ok(())
}

async fn pokemon_to_nt_parallel(
    bar: MultiProgress,
    client: Arc<RustemonClient>,
    tx: mpsc::UnboundedSender<String>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let all_pokemon = match rustemon::pokemon::pokemon::get_all_entries(&client).await {
        Ok(list) => list,
        Err(e) => {
            println!("error getting all pokemon: {:?}", e);
            return Err(e.into());
        }
    };

    let pb = bar.add(ProgressBar::new(all_pokemon.len().try_into().unwrap()));
    let mut index = 0;
    for p in all_pokemon {
        pb.set_message(format!("pokemon #{}", index + 1));
        pb.inc(1);
        index += 1;
        let mut triples: Vec<Triple> = vec![];
        let pokemon_id = NamedNodeRef::new(p.url.as_str())?;
        let pokemon_json = match p.follow(&client).await {
            Ok(list) => list,
            Err(e) => {
                println!("error getting all pokemon: {:?}", e);
                return Err(e.into());
            }
        };
        // Add rdf:type declaration
        triples.push(create_type_triple(pokemon_id, "Pokemon")?);

        triples.push(Triple {
            subject: pokemon_id.into(),
            predicate: NamedNode::new(format!("{POKE}id"))?,
            object: Literal::new_typed_literal(pokemon_json.id.to_string(), xsd::INTEGER).into(),
        });
        triples.push(Triple {
            subject: pokemon_id.into(),
            predicate: NamedNode::new(format!("{POKE}name"))?,
            object: Literal::new_simple_literal(pokemon_json.name).into(),
        });
        if pokemon_json.base_experience.is_some() {
            triples.push(Triple {
                subject: pokemon_id.into(),
                predicate: NamedNode::new(format!("{POKE}baseExperience"))?,
                object: Literal::new_typed_literal(
                    pokemon_json.base_experience.unwrap().to_string(),
                    xsd::INTEGER,
                )
                .into(),
            });
        }
        triples.push(Triple {
            subject: pokemon_id.into(),
            predicate: NamedNode::new(format!("{POKE}height"))?,
            object: Literal::new_typed_literal(pokemon_json.height.to_string(), xsd::INTEGER)
                .into(),
        });

        // is_default
        // order

        triples.push(Triple {
            subject: pokemon_id.into(),
            predicate: NamedNode::new(format!("{POKE}weight"))?,
            object: Literal::new_typed_literal(pokemon_json.weight.to_string(), xsd::INTEGER)
                .into(),
        });

        // types
        for t in pokemon_json.types.clone() {
            triples.push(Triple {
                subject: pokemon_id.into(),
                predicate: NamedNode::new(format!("{POKE}hasType"))?,
                object: NamedNode::new(&t.type_.url)?.into(),
            });
        }

        // abilities
        for a in pokemon_json.abilities.clone() {
            triples.push(Triple {
                subject: pokemon_id.into(),
                predicate: NamedNode::new(format!("{POKE}hasAbility"))?,
                object: NamedNode::new(&a.ability.url)?.into(),
            })
        }

        // moves
        for m in pokemon_json.moves.clone() {
            let move_id = BlankNode::default();
            triples.push(Triple {
                subject: pokemon_id.into(),
                predicate: NamedNode::new(format!("{POKE}pokemonMove"))?,
                object: move_id.as_ref().into(),
            });
            triples.push(Triple {
                subject: move_id.as_ref().into(),
                predicate: NamedNode::new(format!("{POKE}move"))?,
                object: NamedNode::new(m.move_.url)?.into(),
            });
            for v in m.version_group_details {
                let v_id = BlankNode::default();
                triples.push(Triple {
                    subject: move_id.as_ref().into(),
                    predicate: NamedNode::new(format!("{POKE}versionGroupDetails"))?,
                    object: v_id.as_ref().into(),
                });
                triples.push(Triple {
                    subject: v_id.as_ref().into(),
                    predicate: NamedNode::new(format!("{POKE}learnMethod"))?,
                    object: NamedNode::new(v.move_learn_method.url)?.into(),
                });
                triples.push(Triple {
                    subject: v_id.as_ref().into(),
                    predicate: NamedNode::new(format!("{POKE}versionGroup"))?,
                    object: NamedNode::new(v.version_group.url)?.into(),
                });
                triples.push(Triple {
                    subject: v_id.as_ref().into(),
                    predicate: NamedNode::new(format!("{POKE}levelLearnedAt"))?,
                    object: Literal::new_typed_literal(
                        v.level_learned_at.to_string(),
                        xsd::INTEGER,
                    )
                    .into(),
                });
            }
            // self.move_to_nt(m, client).await?;
            // TODO version_group_to_nt
            // TODO move_learn_method_to_nt
        }

        // forms
        for form in pokemon_json.forms.clone() {
            let form_id = NamedNodeRef::new(&form.url)?;
            triples.push(Triple {
                subject: pokemon_id.into(),
                predicate: NamedNode::new(format!("{POKE}hasForm"))?,
                object: form_id.into(),
            });
        }

        // TODO game_indices

        // TODO held_items

        // TODO location_area_encounters

        // TODO past_types

        // TODO sprites

        // species
        let species_id = NamedNodeRef::new(&pokemon_json.species.url)?;
        triples.push(Triple {
            subject: pokemon_id.into(),
            predicate: NamedNode::new(format!("{POKE}species"))?,
            object: species_id.into(),
        });
        for t in triples {
            tx.send(format!("{t} ."))
                .map_err(|e| format!("Send error: {}", e))?
        }
    }

    Ok(())
}

// async fn json_to_rdf(
//     input_json: &str,
//     tmp_dir: &TempDir,
//     triple_file: &mut NamedTempFile,
// ) -> Result<(), std::io::Error> {
//     let mut tmp_file = Builder::new()
//         .tempfile()
//         .expect("could not create temp file");

//     writeln!(tmp_file, "{input_json}").expect("failed writing json to tmp file");

//     let mut binding = Command::new("cat");
//     let echo_child = binding
//         .arg(tmp_file.path().to_str().unwrap())
//         .stdout(Stdio::piped());

//     let output = match echo_child.spawn() {
//         Err(e) => {
//             println!("Error running cat on JSON file: {:?}", e);
//             return Err(e);
//         }
//         Ok(s) => s,
//     };
//     let echo_out = output.stdout.expect("Failed to open echo stdout");

//     // let binding = self.json_validate_path().unwrap();
//     // let json2rdf_call = vec!["-jar", binding.to_str().unwrap(), "https://decisym.ai/data"];
//     let json2rdf_call = vec![
//         "-jar",
//         "/usr/lib/de/atomgraph/json2rdf-1.0.1-jar-with-dependencies.jar",
//         POKE,
//     ];

//     // let mut output: Vec<u8> = vec![];

//     let mut binding = Command::new("java");
//     let sed_child = binding
//         .args(json2rdf_call)
//         .stdin(Stdio::from(echo_out))
//         .stdout(Stdio::piped());

//     let rdf_data = match sed_child.output() {
//         Ok(g) => {
//             if !g.status.success() {
//                 println!(
//                     "json2rdf returned non-zero status code: {}",
//                     String::from_utf8_lossy(&g.stderr)
//                 );
//             };
//             g
//         }
//         Err(e) => {
//             println!("Failed to covert to RDF: {:?}", e);
//             return Err(e);
//         }
//     };
//     writeln!(triple_file, "{}", String::from_utf8_lossy(&rdf_data.stdout))
//         .expect("could not write triples to temp file");

//     Ok(())
// }

#[cfg(test)]
mod tests {

    use super::*;

    #[tokio::test]
    async fn test_rdf() {
        assert!((build_graph().await).is_ok())
    }
}
