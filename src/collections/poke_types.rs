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

pub async fn type_to_nt(
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
    for (index, t) in all_types.into_iter().enumerate() {
        //if !self.types.contains(&t.type_.url) {
        pb.set_message(format!("type #{}", index + 1));
        pb.inc(1);
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
            predicate: NamedNode::new(format!("{SCHEMA}name"))?,
            object: Literal::new_simple_literal(type_json.name).into(),
        });
        triples.push(Triple {
            subject: type_id.into(),
            predicate: NamedNode::new(format!("{SCHEMA}identifier"))?,
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

#[cfg(test)]
mod tests {

    use super::*;

    #[tokio::test]
    async fn test_poke_types() {
        assert!((type_to_nt(
            MultiProgress::new(),
            Arc::new(RustemonClient::default()),
            mpsc::unbounded_channel().0
        )
        .await)
            .is_ok())
    }
}
