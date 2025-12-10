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

pub async fn item_to_nt(
    bar: MultiProgress,
    client: Arc<RustemonClient>,
    tx: mpsc::UnboundedSender<String>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let all_items = match rustemon::items::item::get_all_entries(&client).await {
        Ok(list) => list,
        Err(e) => {
            println!("error getting all items: {:?}", e);
            return Err(e.into());
        }
    };

    let pb = bar.add(ProgressBar::new(all_items.len().try_into().unwrap()));
    for (index, p) in all_items.into_iter().enumerate() {
        pb.set_message(format!("items #{}", index + 1));
        pb.inc(1);
        let mut triples: Vec<Triple> = vec![];
        let item_id = NamedNodeRef::new(p.url.as_str())?;
        let item_json = match p.follow(&client).await {
            Ok(list) => list,
            Err(e) => {
                println!("error getting item info for {}: {e}", &p.url);
                return Err(e.into());
            }
        };
        // Add rdf:type declaration
        triples.push(create_type_triple(item_id, "Item")?);

        triples.push(Triple {
            subject: item_id.into(),
            predicate: NamedNode::new(format!("{SCHEMA}identifier"))?,
            object: Literal::new_typed_literal(item_json.id.to_string(), xsd::INTEGER).into(),
        });
        triples.push(Triple {
            subject: item_id.into(),
            predicate: NamedNode::new(format!("{SCHEMA}name"))?,
            object: Literal::new_simple_literal(item_json.name).into(),
        });

        triples.push(Triple {
            subject: item_id.into(),
            predicate: NamedNode::new(format!("{POKE}cost"))?,
            object: Literal::new_typed_literal(item_json.cost.to_string(), xsd::INTEGER).into(),
        });

        if let Some(power) = item_json.fling_power {
            triples.push(Triple {
                subject: item_id.into(),
                predicate: NamedNode::new(format!("{POKE}flingPower"))?,
                object: Literal::new_typed_literal(power.to_string(), xsd::INTEGER).into(),
            });
        }
        if let Some(effect) = item_json.fling_effect {
            triples.push(Triple {
                subject: item_id.into(),
                predicate: NamedNode::new(format!("{POKE}flingEffect"))?,
                object: NamedNode::new(effect.url)?.into(),
            });
        }

        for attribute in item_json.attributes {
            triples.push(Triple {
                subject: item_id.into(),
                predicate: NamedNode::new(format!("{POKE}hasAttribute"))?,
                object: NamedNode::new(attribute.url)?.into(),
            });
        }

        triples.push(Triple {
            subject: item_id.into(),
            predicate: NamedNode::new(format!("{POKE}itemCategory"))?,
            object: NamedNode::new(item_json.category.url)?.into(),
        });

        for effect in item_json.effect_entries {
            // TODO only english for now
            if effect.language.name == "en" {
                let effect_id = BlankNode::default();
                triples.push(Triple {
                    subject: item_id.into(),
                    predicate: NamedNode::new(format!("{POKE}hasEffect"))?,
                    object: effect_id.as_ref().into(),
                });
                triples.push(Triple {
                    subject: effect_id.as_ref().into(),
                    predicate: NamedNode::new(format!("{SCHEMA}description"))?,
                    object: Literal::new_simple_literal(effect.effect).into(),
                });
                triples.push(Triple {
                    subject: effect_id.as_ref().into(),
                    predicate: NamedNode::new(format!("{POKE}shortEffect"))?,
                    object: Literal::new_simple_literal(effect.short_effect).into(),
                });
            }
        }

        for flavor_text in item_json.flavor_text_entries {
            // TODO only english for now
            if flavor_text.language.name == "en" {
                triples.push(Triple {
                    subject: item_id.into(),
                    predicate: NamedNode::new(format!("{POKE}hasFlavorText"))?,
                    object: Literal::new_simple_literal(flavor_text.text).into(),
                });
            }
        }

        // TODO game_indices
        for index in item_json.game_indices {
            let gi_id = BlankNode::default();
            triples.push(Triple {
                subject: item_id.into(),
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
                object: NamedNode::new(index.generation.url)?.into(),
            });
        }

        for name in item_json.names {
            // TODO only english for now
            if name.language.name == "en" {
                triples.push(Triple {
                    subject: item_id.into(),
                    predicate: NamedNode::new(format!("{POKE}names"))?,
                    object: Literal::new_simple_literal(name.name).into(),
                });
            }
        }

        // TODO sprites
        if let Some(sprite) = item_json.sprites.default {
            // rustemon defines sprite as Option<String>, PokeAPI has image URL:
            // example: https://pokeapi.co/api/v2/item/1
            triples.push(Triple {
                subject: item_id.into(),
                predicate: NamedNode::new(format!("{POKE}defaultSprite"))?,
                object: NamedNode::new(sprite)?.into(),
            });
        }

        for poke in item_json.held_by_pokemon {
            let hold_id = BlankNode::default();
            triples.push(Triple {
                subject: item_id.into(),
                predicate: NamedNode::new(format!("{POKE}heldByPokemon"))?,
                object: hold_id.as_ref().into(),
            });
            triples.push(Triple {
                subject: hold_id.as_ref().into(),
                predicate: NamedNode::new(format!("{POKE}pokemon"))?,
                object: NamedNode::new(poke.pokemon.url)?.into(),
            });
            for version_detail in poke.version_details {
                let version_detail_id = BlankNode::default();
                triples.push(Triple {
                    subject: hold_id.as_ref().into(),
                    predicate: NamedNode::new(format!("{POKE}versionDetail"))?,
                    object: version_detail_id.as_ref().into(),
                });
                triples.push(Triple {
                    subject: version_detail_id.as_ref().into(),
                    predicate: NamedNode::new(format!("{POKE}rarity"))?,
                    object: Literal::new_typed_literal(
                        version_detail.rarity.to_string(),
                        xsd::INTEGER,
                    )
                    .into(),
                });
                triples.push(Triple {
                    subject: version_detail_id.as_ref().into(),
                    predicate: NamedNode::new(format!("{POKE}version"))?,
                    object: NamedNode::new(version_detail.version.url)?.into(),
                });
            }
        }

        if let Some(baby_trigger) = item_json.baby_trigger_for {
            triples.push(Triple {
                subject: item_id.into(),
                predicate: NamedNode::new(format!("{POKE}babyTriggerFor"))?,
                object: NamedNode::new(baby_trigger.url)?.into(),
            });
        }

        // TODO machines

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
    async fn test_items() {
        assert!((item_to_nt(
            MultiProgress::new(),
            Arc::new(RustemonClient::default()),
            mpsc::unbounded_channel().0
        )
        .await)
            .is_ok())
    }
}
