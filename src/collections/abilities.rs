use indicatif::{MultiProgress, ProgressBar};
use oxrdf::vocab::xsd;
use oxrdf::{Literal, NamedNode, NamedNodeRef, Triple};
use rustemon::client::RustemonClient;
use rustemon::Follow;
use std::error::Error;
use std::sync::Arc;
use tokio::sync::mpsc;

use crate::create_type_triple;
use crate::POKE;
use crate::POKEMONKG;
use crate::SCHEMA;

pub async fn ability_to_nt(
    bar: &MultiProgress,
    client: Arc<RustemonClient>,
    tx: mpsc::UnboundedSender<String>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let all_abilities = match rustemon::pokemon::ability::get_all_entries(&client).await {
        Ok(list) => list,
        Err(e) => {
            println!("error getting all abilities: {:?}", e);
            return Err(e.into());
        }
    };
    let len = all_abilities.len();
    let pb = bar.add(
        ProgressBar::new(len.try_into().unwrap())
            .with_style(crate::create_bar_style()),
    );
    pb.finish_with_message("done");
    for (index, p) in all_abilities.into_iter().enumerate() {
        pb.set_message(format!("ability {}/{}", index + 1, len));
        pb.inc(1);
        let mut triples: Vec<Triple> = vec![];
        let ability_id = NamedNodeRef::new(p.url.as_str())?;
        let ability_json = match p.follow(&client).await {
            Ok(list) => list,
            Err(e) => {
                println!("error getting ability info for {}: {e}", &p.url);
                return Err(e.into());
            }
        };
        // Add rdf:type declaration
        triples.push(create_type_triple(ability_id, "Ability")?);

        triples.push(Triple {
            subject: ability_id.into(),
            predicate: NamedNode::new(format!("{SCHEMA}name"))?,
            object: Literal::new_simple_literal(ability_json.name.clone()).into(),
        });
        triples.push(Triple {
            subject: ability_id.into(),
            predicate: NamedNode::new(format!("{SCHEMA}identifier"))?,
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
                    predicate: NamedNode::new(format!("{POKEMONKG}effectDescription"))?,
                    object: Literal::new_simple_literal(v.effect).into(),
                });
                triples.push(Triple {
                    subject: ability_id.into(),
                    predicate: NamedNode::new(format!("{POKEMONKG}effectDescription"))?,
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

        for pokemon in ability_json.pokemon {
            triples.push(Triple {
                subject: ability_id.into(),
                predicate: NamedNode::new(format!("{POKE}mayBeFoundInPokemon"))?,
                object: NamedNodeRef::new(pokemon.pokemon.url.as_str())?.into(),
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
    async fn test_abilities() {
        assert!((ability_to_nt(
            &MultiProgress::new(),
            Arc::new(RustemonClient::default()),
            mpsc::unbounded_channel().0
        )
        .await)
            .is_ok())
    }
}
