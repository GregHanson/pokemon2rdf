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
use crate::SCHEMA;

pub async fn stat_to_nt(
    bar: MultiProgress,
    client: Arc<RustemonClient>,
    tx: mpsc::UnboundedSender<String>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let all_stats = match rustemon::pokemon::stat::get_all_entries(&client).await {
        Ok(list) => list,
        Err(e) => {
            println!("error getting all stats: {:?}", e);
            return Err(e.into());
        }
    };

    let pb = bar.add(ProgressBar::new(all_stats.len().try_into().unwrap()));
    for (index, p) in all_stats.into_iter().enumerate() {
        pb.set_message(format!("stats #{}", index + 1));
        pb.inc(1);
        let mut triples: Vec<Triple> = vec![];
        let stat_id = NamedNodeRef::new(p.url.as_str())?;
        let stat_json = match p.follow(&client).await {
            Ok(list) => list,
            Err(e) => {
                println!("error getting stat info for {}: {e}", &p.url);
                return Err(e.into());
            }
        };
        // Add rdf:type declaration
        triples.push(create_type_triple(stat_id, "Stat")?);

        triples.push(Triple {
            subject: stat_id.into(),
            predicate: NamedNode::new(format!("{SCHEMA}identifier"))?,
            object: Literal::new_typed_literal(stat_json.id.to_string(), xsd::INTEGER).into(),
        });
        triples.push(Triple {
            subject: stat_id.into(),
            predicate: NamedNode::new(format!("{SCHEMA}name"))?,
            object: Literal::new_simple_literal(stat_json.name).into(),
        });

        for decrease in stat_json.affecting_moves.decrease {
            let affect_id = BlankNode::default();
            triples.push(Triple {
                subject: stat_id.into(),
                predicate: NamedNode::new(format!("{POKE}decreasedByMove"))?,
                object: affect_id.as_ref().into(),
            });
            triples.push(Triple {
                subject: affect_id.as_ref().into(),
                predicate: NamedNode::new(format!("{POKE}change"))?,
                object: Literal::new_typed_literal(decrease.change.to_string(), xsd::INTEGER)
                    .into(),
            });
            triples.push(Triple {
                subject: affect_id.as_ref().into(),
                predicate: NamedNode::new(format!("{POKE}move"))?,
                object: NamedNode::new(decrease.move_.url)?.into(),
            });
        }
        for increase in stat_json.affecting_moves.increase {
            let affect_id = BlankNode::default();
            triples.push(Triple {
                subject: stat_id.into(),
                predicate: NamedNode::new(format!("{POKE}increasedByMove"))?,
                object: affect_id.as_ref().into(),
            });
            triples.push(Triple {
                subject: affect_id.as_ref().into(),
                predicate: NamedNode::new(format!("{POKE}change"))?,
                object: Literal::new_typed_literal(increase.change.to_string(), xsd::INTEGER)
                    .into(),
            });
            triples.push(Triple {
                subject: affect_id.as_ref().into(),
                predicate: NamedNode::new(format!("{POKE}move"))?,
                object: NamedNode::new(increase.move_.url)?.into(),
            });
        }

        for nature in stat_json.affecting_natures.increase {
            triples.push(Triple {
                subject: stat_id.into(),
                predicate: NamedNode::new(format!("{POKE}increasedByNature"))?,
                object: NamedNode::new(nature.url)?.into(),
            });
        }

        for nature in stat_json.affecting_natures.decrease {
            triples.push(Triple {
                subject: stat_id.into(),
                predicate: NamedNode::new(format!("{POKE}decreasedByNature"))?,
                object: NamedNode::new(nature.url)?.into(),
            });
        }

        for characteristic in stat_json.characteristics {
            triples.push(Triple {
                subject: stat_id.into(),
                predicate: NamedNode::new(format!("{POKE}hasCharacteristic"))?,
                object: NamedNode::new(characteristic.url)?.into(),
            });
        }

        triples.push(Triple {
            subject: stat_id.into(),
            predicate: NamedNode::new(format!("{POKE}gameIndex"))?,
            object: Literal::new_typed_literal(stat_json.game_index.to_string(), xsd::INTEGER)
                .into(),
        });

        triples.push(Triple {
            subject: stat_id.into(),
            predicate: NamedNode::new(format!("{POKE}isBattleOnly"))?,
            object: Literal::new_typed_literal(stat_json.is_battle_only.to_string(), xsd::BOOLEAN)
                .into(),
        });

        if let Some(move_damage_class) = stat_json.move_damage_class {
            triples.push(Triple {
                subject: stat_id.into(),
                predicate: NamedNode::new(format!("{POKE}moveDamageClass"))?,
                object: NamedNode::new(move_damage_class.url)?.into(),
            });
        }

        for name in stat_json.names {
            // TODO only english for now
            if name.language.name == "en" {
                triples.push(Triple {
                    subject: stat_id.into(),
                    predicate: NamedNode::new(format!("{POKE}names"))?,
                    object: Literal::new_simple_literal(name.name).into(),
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
    async fn test_stats() {
        assert!((stat_to_nt(
            MultiProgress::new(),
            Arc::new(RustemonClient::default()),
            mpsc::unbounded_channel().0
        )
        .await)
            .is_ok())
    }
}
