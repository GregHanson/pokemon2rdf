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

pub async fn pokedex_to_nt(
    bar: &MultiProgress,
    client: Arc<RustemonClient>,
    tx: mpsc::UnboundedSender<String>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let all_pokedexes = match rustemon::games::pokedex::get_all_entries(&client).await {
        Ok(list) => list,
        Err(e) => {
            println!("error getting all pokedexes: {:?}", e);
            return Err(e.into());
        }
    };
    let len = all_pokedexes.len();
    let pb =
        bar.add(ProgressBar::new(len.try_into().unwrap()).with_style(crate::create_bar_style()));
    pb.finish_with_message("done");
    for (index, p) in all_pokedexes.into_iter().enumerate() {
        pb.set_message(format!("pokedexes {}/{}", index + 1, len));
        pb.inc(1);
        let mut triples: Vec<Triple> = vec![];
        let pokedex_id = NamedNodeRef::new(p.url.as_str())?;
        let pokedex_json = match p.follow(&client).await {
            Ok(list) => list,
            Err(e) => {
                println!("error getting pokedex info for {}: {e}", &p.url);
                return Err(e.into());
            }
        };
        // Add rdf:type declaration
        triples.push(create_type_triple(pokedex_id, "Pokedex")?);

        triples.push(Triple {
            subject: pokedex_id.into(),
            predicate: NamedNode::new(format!("{SCHEMA}identifier"))?,
            object: Literal::new_typed_literal(pokedex_json.id.to_string(), xsd::INTEGER).into(),
        });
        triples.push(Triple {
            subject: pokedex_id.into(),
            predicate: NamedNode::new(format!("{SCHEMA}name"))?,
            object: Literal::new_simple_literal(pokedex_json.name).into(),
        });

        // TODO is_main_series

        for description in pokedex_json.descriptions {
            // TODO only english for now
            if description.language.name == "en" {
                triples.push(Triple {
                    subject: pokedex_id.into(),
                    predicate: NamedNode::new(format!("{SCHEMA}description"))?,
                    object: Literal::new_simple_literal(description.description).into(),
                });
            }
        }

        for name in pokedex_json.names {
            // TODO only english for now
            if name.language.name == "en" {
                triples.push(Triple {
                    subject: pokedex_id.into(),
                    predicate: NamedNode::new(format!("{POKE}names"))?,
                    object: Literal::new_simple_literal(name.name).into(),
                });
            }
        }

        for (i, entry) in pokedex_json.pokemon_entries.into_iter().enumerate() {
            let entry_id = BlankNode::new(format!("pokedex{}_entry{}", pokedex_json.id, i))?;
            triples.push(Triple {
                subject: pokedex_id.into(),
                predicate: NamedNode::new(format!("{POKE}hasPokedexEntry"))?,
                object: entry_id.as_ref().into(),
            });
            triples.push(Triple {
                subject: entry_id.as_ref().into(),
                predicate: NamedNode::new(format!("{POKE}entryNumber"))?,
                object: Literal::new_typed_literal(entry.entry_number.to_string(), xsd::INTEGER)
                    .into(),
            });
            triples.push(Triple {
                subject: entry_id.as_ref().into(),
                predicate: NamedNode::new(format!("{POKE}species"))?,
                object: NamedNode::new(entry.pokemon_species.url)?.into(),
            });
        }

        if let Some(region) = pokedex_json.region {
            triples.push(Triple {
                subject: pokedex_id.into(),
                predicate: NamedNode::new(format!("{POKE}region"))?,
                object: NamedNode::new(region.url)?.into(),
            });
        }

        for group in pokedex_json.version_groups {
            triples.push(Triple {
                subject: pokedex_id.into(),
                predicate: NamedNode::new(format!("{POKE}versionGroup"))?,
                object: NamedNode::new(group.url)?.into(),
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
    async fn test_pokedex() {
        assert!((pokedex_to_nt(
            &MultiProgress::new(),
            Arc::new(RustemonClient::default()),
            mpsc::unbounded_channel().0
        )
        .await)
            .is_ok())
    }
}
