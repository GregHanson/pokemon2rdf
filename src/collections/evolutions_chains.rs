use indicatif::{MultiProgress, ProgressBar};
use oxrdf::vocab::xsd;
use oxrdf::{BlankNode, Literal, NamedNode, NamedNodeRef, Triple};
use rustemon::client::RustemonClient;
use rustemon::model::evolution::ChainLink;
use rustemon::Follow;
use std::error::Error;
use std::sync::Arc;
use tokio::sync::mpsc;

use crate::create_type_triple;
use crate::POKE;
use crate::POKEMONKG;
use crate::SCHEMA;

pub async fn evolution_chain_to_nt(
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
    for (index, p) in chains.into_iter().enumerate() {
        pb.set_message(format!("evolution chain #{}", index + 1));
        pb.inc(1);
        let mut triples: Vec<Triple> = vec![];
        let chain_id = NamedNodeRef::new(p.url.as_str())?;
        let chain_json = match p.follow(&client).await {
            Ok(list) => list,
            Err(e) => {
                println!("error getting evolution chain info for {}: {e}", &p.url);
                return Err(e.into());
            }
        };
        // Add rdf:type declaration
        triples.push(create_type_triple(chain_id, "EvolutionChain")?);

        triples.push(Triple {
            subject: chain_id.into(),
            predicate: NamedNode::new(format!("{SCHEMA}identifier"))?,
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
            triples.extend_from_slice(&chain_link_to_nt(chain_id, evolve)?);
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
    for (i, detail) in link.evolution_details.clone().into_iter().enumerate() {
        let detail_id = BlankNode::new(format!("link{}_evolutionDetail{}", link_id, i))?;
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
                predicate: NamedNode::new(format!("{POKEMONKG}minLevelToLearn"))?,
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

#[cfg(test)]
mod tests {

    use super::*;

    #[tokio::test]
    async fn test_evolution_chains() {
        assert!((evolution_chain_to_nt(
            MultiProgress::new(),
            Arc::new(RustemonClient::default()),
            mpsc::unbounded_channel().0
        )
        .await)
            .is_ok())
    }
}
