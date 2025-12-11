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
use crate::SCHEMA;

pub async fn generation_to_nt(
    bar: &MultiProgress,
    client: Arc<RustemonClient>,
    tx: mpsc::UnboundedSender<String>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let all_generations = match rustemon::games::generation::get_all_entries(&client).await {
        Ok(list) => list,
        Err(e) => {
            println!("error getting all generations: {:?}", e);
            return Err(e.into());
        }
    };
    let len = all_generations.len();
    let pb =
        bar.add(ProgressBar::new(len.try_into().unwrap()).with_style(crate::create_bar_style()));
    pb.finish_with_message("done");
    for (index, p) in all_generations.into_iter().enumerate() {
        pb.set_message(format!("generation {}/{}", index + 1, len));
        pb.inc(1);
        let mut triples: Vec<Triple> = vec![];
        let gen_id = NamedNodeRef::new(p.url.as_str())?;
        let gen_json = match p.follow(&client).await {
            Ok(list) => list,
            Err(e) => {
                println!("error getting generation info for {}: {e}", &p.url);
                return Err(e.into());
            }
        };
        // Add rdf:type declaration
        triples.push(create_type_triple(gen_id, "Generation")?);

        triples.push(Triple {
            subject: gen_id.into(),
            predicate: NamedNode::new(format!("{SCHEMA}identifier"))?,
            object: Literal::new_typed_literal(gen_json.id.to_string(), xsd::INTEGER).into(),
        });
        triples.push(Triple {
            subject: gen_id.into(),
            predicate: NamedNode::new(format!("{SCHEMA}name"))?,
            object: Literal::new_simple_literal(gen_json.name).into(),
        });
        // abilities
        for a in gen_json.abilities {
            triples.push(Triple {
                subject: gen_id.into(),
                predicate: NamedNode::new(format!("{POKE}hasNewAbiltiy"))?,
                object: NamedNode::new(a.url)?.into(),
            });
        }
        // names
        for n in gen_json.names {
            // TODO only english for now
            if n.language.name == "en" {
                triples.push(Triple {
                    subject: gen_id.into(),
                    predicate: NamedNode::new(format!("{POKE}names"))?,
                    object: Literal::new_simple_literal(n.name).into(),
                });
            }
        }
        // main_region
        triples.push(Triple {
            subject: gen_id.into(),
            predicate: NamedNode::new(format!("{POKE}region"))?,
            object: NamedNode::new(&gen_json.main_region.url)?.into(),
        });
        // moves
        for m in gen_json.moves {
            triples.push(Triple {
                subject: gen_id.into(),
                predicate: NamedNode::new(format!("{POKE}hasNewMove"))?,
                object: NamedNode::new(m.url)?.into(),
            });
        }
        // pokemon_species
        for s in gen_json.pokemon_species {
            triples.push(Triple {
                subject: gen_id.into(),
                predicate: NamedNode::new(format!("{POKE}hasNewSpecies"))?,
                object: NamedNode::new(s.url)?.into(),
            });
        }
        // types
        for t in gen_json.types {
            triples.push(Triple {
                subject: gen_id.into(),
                predicate: NamedNode::new(format!("{POKE}hasNewType"))?,
                object: NamedNode::new(t.url)?.into(),
            });
        }
        // version_groups
        for v in gen_json.version_groups {
            triples.push(Triple {
                subject: gen_id.into(),
                predicate: NamedNode::new(format!("{POKE}hasNewVersionGroup"))?,
                object: NamedNode::new(v.url)?.into(),
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
    async fn test_generations() {
        assert!((generation_to_nt(
            &MultiProgress::new(),
            Arc::new(RustemonClient::default()),
            mpsc::unbounded_channel().0
        )
        .await)
            .is_ok())
    }
}
