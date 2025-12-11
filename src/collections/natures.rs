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

pub async fn nature_to_nt(
    bar: MultiProgress,
    client: Arc<RustemonClient>,
    tx: mpsc::UnboundedSender<String>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let all_natures = match rustemon::pokemon::nature::get_all_entries(&client).await {
        Ok(list) => list,
        Err(e) => {
            println!("error getting all natures: {:?}", e);
            return Err(e.into());
        }
    };

    let pb = bar.add(ProgressBar::new(all_natures.len().try_into().unwrap()));
    for (index, p) in all_natures.into_iter().enumerate() {
        pb.set_message(format!("natures #{}", index + 1));
        pb.inc(1);
        let mut triples: Vec<Triple> = vec![];
        let nature_id = NamedNodeRef::new(p.url.as_str())?;
        let nature_json = match p.follow(&client).await {
            Ok(list) => list,
            Err(e) => {
                println!("error getting nature info for {}: {e}", &p.url);
                return Err(e.into());
            }
        };
        // Add rdf:type declaration
        triples.push(create_type_triple(nature_id, "Nature")?);

        triples.push(Triple {
            subject: nature_id.into(),
            predicate: NamedNode::new(format!("{SCHEMA}identifier"))?,
            object: Literal::new_typed_literal(nature_json.id.to_string(), xsd::INTEGER).into(),
        });
        triples.push(Triple {
            subject: nature_id.into(),
            predicate: NamedNode::new(format!("{SCHEMA}name"))?,
            object: Literal::new_simple_literal(nature_json.name).into(),
        });

        if let Some(decrease) = nature_json.decreased_stat {
            triples.push(Triple {
                subject: nature_id.into(),
                predicate: NamedNode::new(format!("{POKE}decreasedStat"))?,
                object: NamedNode::new(decrease.url)?.into(),
            });
        }
        if let Some(increase) = nature_json.increased_stat {
            triples.push(Triple {
                subject: nature_id.into(),
                predicate: NamedNode::new(format!("{POKE}increasedStat"))?,
                object: NamedNode::new(increase.url)?.into(),
            });
        }

        if let Some(hates_flavor) = nature_json.hates_flavor {
            triples.push(Triple {
                subject: nature_id.into(),
                predicate: NamedNode::new(format!("{POKE}hatesFlavor"))?,
                object: NamedNode::new(hates_flavor.url)?.into(),
            });
        }
        if let Some(likes_flavor) = nature_json.likes_flavor {
            triples.push(Triple {
                subject: nature_id.into(),
                predicate: NamedNode::new(format!("{POKE}likesFlavor"))?,
                object: NamedNode::new(likes_flavor.url)?.into(),
            });
        }

        for (i, preference) in nature_json
            .move_battle_style_preferences
            .into_iter()
            .enumerate()
        {
            let pref_id = BlankNode::new(format!(
                "nature{}_battlestylepreference{}",
                nature_json.id, i
            ))?;
            triples.push(Triple {
                subject: nature_id.into(),
                predicate: NamedNode::new(format!("{POKE}hasMoveBattleStylePreference"))?,
                object: pref_id.as_ref().into(),
            });
            triples.push(Triple {
                subject: pref_id.as_ref().into(),
                predicate: NamedNode::new(format!("{POKE}lowHpPreference"))?,
                object: Literal::new_typed_literal(
                    preference.low_hp_preference.to_string(),
                    xsd::INTEGER,
                )
                .into(),
            });
            triples.push(Triple {
                subject: pref_id.as_ref().into(),
                predicate: NamedNode::new(format!("{POKE}highHpPreference"))?,
                object: Literal::new_typed_literal(
                    preference.high_hp_preference.to_string(),
                    xsd::INTEGER,
                )
                .into(),
            });
            triples.push(Triple {
                subject: pref_id.as_ref().into(),
                predicate: NamedNode::new(format!("{POKE}moveBattleStyle"))?,
                object: NamedNode::new(preference.move_battle_style.url)?.into(),
            });
        }

        for (i, pokeathlon_stat) in nature_json.pokeathlon_stat_changes.into_iter().enumerate() {
            let stat_change_id = BlankNode::new(format!(
                "nature{}_pokeathlonstatchange{}",
                nature_json.id, i
            ))?;
            triples.push(Triple {
                subject: nature_id.into(),
                predicate: NamedNode::new(format!("{POKE}hasPokeathlonStatChange"))?,
                object: stat_change_id.as_ref().into(),
            });
            triples.push(Triple {
                subject: stat_change_id.as_ref().into(),
                predicate: NamedNode::new(format!("{POKE}maxChange"))?,
                object: Literal::new_typed_literal(
                    pokeathlon_stat.max_change.to_string(),
                    xsd::INTEGER,
                )
                .into(),
            });
            triples.push(Triple {
                subject: stat_change_id.as_ref().into(),
                predicate: NamedNode::new(format!("{POKE}pokeathlonStat"))?,
                object: NamedNode::new(pokeathlon_stat.pokeathlon_stat.url)?.into(),
            });
        }

        for name in nature_json.names {
            // TODO only english for now
            if name.language.name == "en" {
                triples.push(Triple {
                    subject: nature_id.into(),
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
    async fn test_natures() {
        assert!((nature_to_nt(
            MultiProgress::new(),
            Arc::new(RustemonClient::default()),
            mpsc::unbounded_channel().0
        )
        .await)
            .is_ok())
    }
}
