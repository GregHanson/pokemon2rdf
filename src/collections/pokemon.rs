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

pub async fn pokemon_to_nt(
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
    for (index, p) in all_pokemon.into_iter().enumerate() {
        pb.set_message(format!("pokemon #{}", index + 1));
        pb.inc(1);
        let mut triples: Vec<Triple> = vec![];
        let pokemon_id = NamedNodeRef::new(p.url.as_str())?;
        let pokemon_json = match p.follow(&client).await {
            Ok(list) => list,
            Err(e) => {
                println!("error getting pokemon info for {}: {e}", &p.url);
                return Err(e.into());
            }
        };
        // Add rdf:type declaration
        triples.push(create_type_triple(pokemon_id, "Pokemon")?);

        triples.push(Triple {
            subject: pokemon_id.into(),
            predicate: NamedNode::new(format!("{SCHEMA}identifier"))?,
            object: Literal::new_typed_literal(pokemon_json.id.to_string(), xsd::INTEGER).into(),
        });
        triples.push(Triple {
            subject: pokemon_id.into(),
            predicate: NamedNode::new(format!("{SCHEMA}name"))?,
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

        triples.push(Triple {
            subject: pokemon_id.into(),
            predicate: NamedNode::new(format!("{POKE}order"))?,
            object: Literal::new_typed_literal(pokemon_json.order.to_string(), xsd::INTEGER).into(),
        });

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
                predicate: NamedNode::new(format!("{POKEMONKG}hasType"))?,
                object: NamedNode::new(&t.type_.url)?.into(),
            });
        }

        // abilities
        for a in pokemon_json.abilities.clone() {
            triples.push(Triple {
                subject: pokemon_id.into(),
                predicate: NamedNode::new(format!("{POKEMONKG}mayHaveAbility"))?,
                object: NamedNode::new(&a.ability.url)?.into(),
            })
        }

        // moves
        for (i, m) in pokemon_json.moves.into_iter().enumerate() {
            let move_id = BlankNode::new(format!("pokemon{}_move{}", pokemon_json.id, i))?;
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
            for (j, v) in m.version_group_details.into_iter().enumerate() {
                let v_id = BlankNode::new(format!(
                    "pokemon{}_move{}_versionGroup{}",
                    pokemon_json.id, i, j
                ))?;
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

        for (i, index) in pokemon_json.game_indices.into_iter().enumerate() {
            let gi_id = BlankNode::new(format!("pokemon{}_gameIndex{}", pokemon_json.id, i))?;
            triples.push(Triple {
                subject: pokemon_id.into(),
                predicate: NamedNode::new(format!("{POKE}gameIndex"))?,
                object: gi_id.as_ref().into(),
            });
            triples.push(Triple {
                subject: gi_id.as_ref().into(),
                predicate: NamedNode::new(format!("{POKE}index"))?,
                object: Literal::new_typed_literal(index.game_index.to_string(), xsd::INTEGER)
                    .into(),
            });
            triples.push(Triple {
                subject: gi_id.as_ref().into(),
                predicate: NamedNode::new(format!("{POKE}generation"))?,
                object: NamedNode::new(index.version.url)?.into(),
            });
        }

        for (i, item) in pokemon_json.held_items.into_iter().enumerate() {
            for (j, version_detail) in item.version_details.into_iter().enumerate() {
                let v_id = BlankNode::new(format!(
                    "pokemon{}_heldItem{}_versionDetail{}",
                    pokemon_json.id, i, j
                ))?;
                triples.push(Triple {
                    subject: pokemon_id.into(),
                    predicate: NamedNode::new(format!("{POKE}mayHoldItem"))?,
                    object: v_id.as_ref().into(),
                });
                triples.push(Triple {
                    subject: v_id.as_ref().into(),
                    predicate: NamedNode::new(format!("{POKE}item"))?,
                    object: NamedNode::new(item.item.url.clone())?.into(),
                });
                triples.push(Triple {
                    subject: v_id.as_ref().into(),
                    predicate: NamedNode::new(format!("{POKE}version"))?,
                    object: NamedNode::new(version_detail.version.url)?.into(),
                });
                triples.push(Triple {
                    subject: v_id.as_ref().into(),
                    predicate: NamedNode::new(format!("{POKE}rarity"))?,
                    object: Literal::new_typed_literal(
                        version_detail.rarity.to_string(),
                        xsd::INTEGER,
                    )
                    .into(),
                });
            }
        }

        // TODO location_area_encounters
        // rustemon has String but PokeApi returns a URL
        // example: https://pokeapi.co/api/v2/pokemon/10
        let lae_id = NamedNode::new(pokemon_json.location_area_encounters.clone())?;
        triples.push(Triple {
            subject: pokemon_id.into(),
            predicate: NamedNode::new(format!("{POKE}hasLocationAreaEncounter"))?,
            object: lae_id.as_ref().into(),
        });

        let location_area_encounters =
            rustemon::pokemon::pokemon::encounters::get_by_id(pokemon_json.id, &client)
                .await
                .map_err(|e| {
                    format!(
                        "failed to get pokemon location area encounters {}: {e}",
                        pokemon_json.location_area_encounters
                    )
                })?;
        for (i, location_area_encounter) in location_area_encounters.into_iter().enumerate() {
            for (j, version_detail) in location_area_encounter
                .version_details
                .into_iter()
                .enumerate()
            {
                let vd_id = BlankNode::new(format!(
                    "pokemon{}_locationAreaEncounter{}_versionDetail{}",
                    pokemon_json.id, i, j
                ))?;

                triples.push(Triple {
                    subject: lae_id.as_ref().into(),
                    predicate: NamedNode::new(format!("{POKE}locationAreaEncounter"))?,
                    object: vd_id.as_ref().into(),
                });
                triples.push(Triple {
                    subject: vd_id.as_ref().into(),
                    predicate: NamedNode::new(format!("{POKE}locationArea"))?,
                    object: NamedNode::new(location_area_encounter.location_area.url.clone())?
                        .into(),
                });
                triples.push(Triple {
                    subject: vd_id.as_ref().into(),
                    predicate: NamedNode::new(format!("{POKE}version"))?,
                    object: NamedNode::new(version_detail.version.url)?.into(),
                });
                triples.push(Triple {
                    subject: vd_id.as_ref().into(),
                    predicate: NamedNode::new(format!("{POKE}maxChance"))?,
                    object: Literal::new_typed_literal(
                        version_detail.max_chance.to_string(),
                        xsd::INTEGER,
                    )
                    .into(),
                });
                for encounter_detail in version_detail.encounter_details {
                    let ed_id = BlankNode::default();
                    triples.push(Triple {
                        subject: vd_id.as_ref().into(),
                        predicate: NamedNode::new(format!("{POKE}encounterDetail"))?,
                        object: ed_id.as_ref().into(),
                    });
                    triples.push(Triple {
                        subject: ed_id.as_ref().into(),
                        predicate: NamedNode::new(format!("{POKE}method"))?,
                        object: NamedNode::new(encounter_detail.method.url)?.into(),
                    });
                    triples.push(Triple {
                        subject: ed_id.as_ref().into(),
                        predicate: NamedNode::new(format!("{POKE}chance"))?,
                        object: Literal::new_typed_literal(
                            encounter_detail.chance.to_string(),
                            xsd::INTEGER,
                        )
                        .into(),
                    });
                    triples.push(Triple {
                        subject: ed_id.as_ref().into(),
                        predicate: NamedNode::new(format!("{POKE}minLevel"))?,
                        object: Literal::new_typed_literal(
                            encounter_detail.min_level.to_string(),
                            xsd::INTEGER,
                        )
                        .into(),
                    });
                    triples.push(Triple {
                        subject: ed_id.as_ref().into(),
                        predicate: NamedNode::new(format!("{POKE}maxLevel"))?,
                        object: Literal::new_typed_literal(
                            encounter_detail.max_level.to_string(),
                            xsd::INTEGER,
                        )
                        .into(),
                    });
                    for condition in encounter_detail.condition_values {
                        triples.push(Triple {
                            subject: ed_id.as_ref().into(),
                            predicate: NamedNode::new(format!("{POKE}hasCondition"))?,
                            object: NamedNode::new(condition.url)?.into(),
                        });
                    }
                }
            }
        }

        // TODO past_types
        for (i, p_type) in pokemon_json.past_types.into_iter().enumerate() {
            for (j, t) in p_type.types.into_iter().enumerate() {
                let past_type_id = BlankNode::new(format!(
                    "pokemon{}_pastType{}_type{}",
                    pokemon_json.id, i, j
                ))?;
                triples.push(Triple {
                    subject: pokemon_id.into(),
                    predicate: NamedNode::new(format!("{POKE}pastType"))?,
                    object: past_type_id.as_ref().into(),
                });
                triples.push(Triple {
                    subject: past_type_id.as_ref().into(),
                    predicate: NamedNode::new(format!("{POKEMONKG}hasType"))?,
                    object: NamedNode::new(t.type_.url)?.into(),
                });
                triples.push(Triple {
                    subject: past_type_id.as_ref().into(),
                    predicate: NamedNode::new(format!("{POKE}generation"))?,
                    object: NamedNode::new(p_type.generation.url.clone())?.into(),
                });
            }
        }

        // TODO sprites
        // one node with all sprite URLs as properties? Declare media type as image/png?
        if let Some(front_default) = pokemon_json.sprites.front_default {
            triples.push(Triple {
                subject: pokemon_id.into(),
                predicate: NamedNode::new(format!("{POKE}frontDefaultSprite"))?,
                object: NamedNode::new(front_default)?.into(),
            });
        }
        if let Some(back_default) = pokemon_json.sprites.back_default {
            triples.push(Triple {
                subject: pokemon_id.into(),
                predicate: NamedNode::new(format!("{POKE}backDefaultSprite"))?,
                object: NamedNode::new(back_default)?.into(),
            });
        }
        if let Some(front_shiny) = pokemon_json.sprites.front_shiny {
            triples.push(Triple {
                subject: pokemon_id.into(),
                predicate: NamedNode::new(format!("{POKE}frontShinySprite"))?,
                object: NamedNode::new(front_shiny)?.into(),
            });
        }
        if let Some(back_shiny) = pokemon_json.sprites.back_shiny {
            triples.push(Triple {
                subject: pokemon_id.into(),
                predicate: NamedNode::new(format!("{POKE}backShinySprite"))?,
                object: NamedNode::new(back_shiny)?.into(),
            });
        }
        if let Some(front_female) = pokemon_json.sprites.front_female {
            triples.push(Triple {
                subject: pokemon_id.into(),
                predicate: NamedNode::new(format!("{POKE}frontFemaleSprite"))?,
                object: NamedNode::new(front_female)?.into(),
            });
        }
        if let Some(back_female) = pokemon_json.sprites.back_female {
            triples.push(Triple {
                subject: pokemon_id.into(),
                predicate: NamedNode::new(format!("{POKE}backFemaleSprite"))?,
                object: NamedNode::new(back_female)?.into(),
            });
        }
        if let Some(front_female_shiny) = pokemon_json.sprites.front_shiny_female {
            triples.push(Triple {
                subject: pokemon_id.into(),
                predicate: NamedNode::new(format!("{POKE}frontFemaleShinySprite"))?,
                object: NamedNode::new(front_female_shiny)?.into(),
            });
        }
        if let Some(back_female_shiny) = pokemon_json.sprites.back_shiny_female {
            triples.push(Triple {
                subject: pokemon_id.into(),
                predicate: NamedNode::new(format!("{POKE}backFemaleShinySprite"))?,
                object: NamedNode::new(back_female_shiny)?.into(),
            });
        }

        // OtherSprites
        // VersionSprites

        // species
        let species_id = NamedNodeRef::new(&pokemon_json.species.url)?;
        triples.push(Triple {
            subject: pokemon_id.into(),
            predicate: NamedNode::new(format!("{POKE}species"))?,
            object: species_id.into(),
        });

        for (i, stat) in pokemon_json.stats.into_iter().enumerate() {
            let stat_id = BlankNode::new(format!("pokemon{}_stat{}", pokemon_json.id, i))?;
            triples.push(Triple {
                subject: pokemon_id.into(),
                predicate: NamedNode::new(format!("{POKE}pokemonStat"))?,
                object: stat_id.as_ref().into(),
            });
            triples.push(Triple {
                subject: stat_id.as_ref().into(),
                predicate: NamedNode::new(format!("{POKE}stat"))?,
                object: NamedNode::new(stat.stat.url)?.into(),
            });
            triples.push(Triple {
                subject: stat_id.as_ref().into(),
                predicate: NamedNode::new(format!("{POKE}baseStat"))?,
                object: Literal::new_typed_literal(stat.base_stat.to_string(), xsd::INTEGER).into(),
            });
            triples.push(Triple {
                subject: stat_id.as_ref().into(),
                predicate: NamedNode::new(format!("{POKE}effort"))?,
                object: Literal::new_typed_literal(stat.effort.to_string(), xsd::INTEGER).into(),
            });
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
    async fn test_pokemon() {
        assert!((pokemon_to_nt(
            MultiProgress::new(),
            Arc::new(RustemonClient::default()),
            mpsc::unbounded_channel().0
        )
        .await)
            .is_ok())
    }
}
