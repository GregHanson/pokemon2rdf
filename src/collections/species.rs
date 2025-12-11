use indicatif::{MultiProgress, ProgressBar};
use oxrdf::vocab::xsd;
use oxrdf::{BlankNode, Literal, NamedNode, NamedNodeRef, Triple};
use rustemon::client::RustemonClient;
use rustemon::Follow;
use std::error::Error;
use std::sync::Arc;
use tokio::sync::mpsc;

use crate::create_type_triple;
use crate::POKE;
use crate::POKEMONKG;
use crate::SCHEMA;

pub async fn species_to_nt(
    bar: &MultiProgress,
    client: Arc<RustemonClient>,
    tx: mpsc::UnboundedSender<String>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let all_species = match rustemon::pokemon::pokemon_species::get_all_entries(&client).await {
        Ok(list) => list,
        Err(e) => {
            println!("error getting all species: {:?}", e);
            return Err(e.into());
        }
    };
    let len = all_species.len();
    let pb =
        bar.add(ProgressBar::new(len.try_into().unwrap()).with_style(crate::create_bar_style()));
    pb.finish_with_message("done");
    for (index, p) in all_species.into_iter().enumerate() {
        pb.set_message(format!("species {}/{}", index + 1, len));
        pb.inc(1);
        let mut triples: Vec<Triple> = vec![];
        let species_id = NamedNodeRef::new(p.url.as_str())?;
        let species_json = match p.follow(&client).await {
            Ok(list) => list,
            Err(e) => {
                println!("error getting species info for {}: {e}", &p.url);
                return Err(e.into());
            }
        };
        // Add rdf:type declaration
        triples.push(create_type_triple(species_id, "PokemonSpecies")?);

        triples.push(Triple {
            subject: species_id.into(),
            predicate: NamedNode::new(format!("{SCHEMA}identifier"))?,
            object: Literal::new_typed_literal(species_json.id.to_string(), xsd::INTEGER).into(),
        });
        triples.push(Triple {
            subject: species_id.into(),
            predicate: NamedNode::new(format!("{SCHEMA}name"))?,
            object: Literal::new_simple_literal(species_json.name).into(),
        });
        triples.push(Triple {
            subject: species_id.into(),
            predicate: NamedNode::new(format!("{POKE}order"))?,
            object: Literal::new_typed_literal(species_json.order.to_string(), xsd::INTEGER).into(),
        });
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
        for (i, pokedex) in species_json.pokedex_numbers.into_iter().enumerate() {
            let pdx_id = BlankNode::new(format!("species{}_pokedexnumber{}", species_json.id, i))?;
            triples.push(Triple {
                subject: species_id.into(),
                predicate: NamedNode::new(format!("{POKE}hasPokedexNumber"))?,
                object: pdx_id.as_ref().into(),
            });
            triples.push(Triple {
                subject: pdx_id.as_ref().into(),
                predicate: NamedNode::new(format!("{POKE}entryNumber"))?,
                object: Literal::new_typed_literal(pokedex.entry_number.to_string(), xsd::INTEGER)
                    .into(),
            });
            triples.push(Triple {
                subject: pdx_id.as_ref().into(),
                predicate: NamedNode::new(format!("{POKE}pokedex"))?,
                object: NamedNode::new(pokedex.pokedex.url)?.into(),
            });
        }

        // TODO egg_groups
        for e in species_json.egg_groups {
            triples.push(Triple {
                subject: species_id.into(),
                predicate: NamedNode::new(format!("{POKEMONKG}inEggGroup"))?,
                object: NamedNodeRef::new(e.url.as_str())?.into(),
            });
        }
        // TODO color
        triples.push(Triple {
            subject: species_id.into(),
            predicate: NamedNode::new(format!("{POKEMONKG}hasColour"))?,
            object: NamedNodeRef::new(&species_json.color.url)?.into(),
        });
        // TODO shape
        if let Some(shape) = species_json.shape {
            triples.push(Triple {
                subject: species_id.into(),
                predicate: NamedNode::new(format!("{POKEMONKG}hasShape"))?,
                object: NamedNodeRef::new(&shape.url)?.into(),
            });
        }
        // TODO evolves_from_species
        if let Some(s) = species_json.evolves_from_species {
            triples.push(Triple {
                subject: species_id.into(),
                predicate: NamedNode::new(format!("{POKEMONKG}evolvesFrom"))?,
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
                predicate: NamedNode::new(format!("{POKEMONKG}foundIn"))?,
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
        for (i, enc) in species_json.pal_park_encounters.into_iter().enumerate() {
            let enc_id =
                BlankNode::new(format!("species{}_palparkencounter{}", species_json.id, i))?;
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
        for (i, f) in species_json.flavor_text_entries.into_iter().enumerate() {
            // TODO only english for now
            if f.language.name == "en" {
                let flavor_id =
                    BlankNode::new(format!("species{}_flavortext{}", species_json.id, i))?;
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
                    predicate: NamedNode::new(format!("{POKEMONKG}hasGenus"))?,
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

#[cfg(test)]
mod tests {

    use super::*;

    #[tokio::test]
    async fn test_species() {
        assert!((species_to_nt(
            &MultiProgress::new(),
            Arc::new(RustemonClient::default()),
            mpsc::unbounded_channel().0
        )
        .await)
            .is_ok())
    }
}
