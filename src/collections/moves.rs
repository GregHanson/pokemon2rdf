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

pub async fn move_to_nt(
    bar: MultiProgress,
    client: Arc<RustemonClient>,
    tx: mpsc::UnboundedSender<String>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let all_moves = match rustemon::moves::move_::get_all_entries(&client).await {
        Ok(list) => list,
        Err(e) => {
            println!("error getting all moves: {:?}", e);
            return Err(e.into());
        }
    };
    let pb = bar.add(ProgressBar::new(all_moves.len().try_into().unwrap()));
    for (index, m) in all_moves.into_iter().enumerate() {
        pb.set_message(format!("move #{}", index + 1));
        pb.inc(1);
        let mut triples = vec![];
        let move_id = NamedNodeRef::new(&m.url)?;
        // Add rdf:type declaration
        triples.push(create_type_triple(move_id, "Move")?);

        triples.push(Triple {
            subject: move_id.into(),
            predicate: NamedNode::new(format!("{SCHEMA}name"))?,
            object: Literal::new_simple_literal(m.name.clone()).into(),
        });
        let move_json = match m.follow(&client).await {
            Ok(v) => v,
            Err(e) => {
                eprintln!("Error getting move info for {}: {e}", &m.url);
                return Err(e.into());
            }
        };
        triples.push(Triple {
            subject: move_id.into(),
            predicate: NamedNode::new(format!("{SCHEMA}identifier"))?,
            object: Literal::new_typed_literal(move_json.id.to_string(), xsd::INTEGER).into(),
        });
        if move_json.accuracy.is_some() {
            triples.push(Triple {
                subject: move_id.into(),
                predicate: NamedNode::new(format!("{POKEMONKG}accuracy"))?,
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
                predicate: NamedNode::new(format!("{POKEMONKG}basePowerPoints"))?,
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
                predicate: NamedNode::new(format!("{POKEMONKG}basePower"))?,
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
                    predicate: NamedNode::new(format!("{POKEMONKG}effectDescription"))?,
                    object: Literal::new_simple_literal(effect.effect).into(),
                });
                triples.push(Triple {
                    subject: move_id.into(),
                    predicate: NamedNode::new(format!("{POKEMONKG}effectDescription"))?,
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
                predicate: NamedNode::new(format!("{SCHEMA}name"))?,
                object: Literal::new_simple_literal(stat.stat.name).into(),
            });
            triples.push(Triple {
                subject: stat_change_id.as_ref().into(),
                predicate: NamedNode::new(format!("{POKE}change"))?,
                object: Literal::new_typed_literal(stat.change.to_string(), xsd::INTEGER).into(),
            });
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
            predicate: NamedNode::new(format!("{POKEMONKG}hasType"))?,
            object: NamedNode::new(move_json.type_.url)?.into(),
        });

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
    async fn test_moves() {
        assert!((move_to_nt(
            MultiProgress::new(),
            Arc::new(RustemonClient::default()),
            mpsc::unbounded_channel().0
        )
        .await)
            .is_ok())
    }
}
