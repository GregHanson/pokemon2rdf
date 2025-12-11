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

pub async fn shape_to_nt(
    bar: &MultiProgress,
    client: Arc<RustemonClient>,
    tx: mpsc::UnboundedSender<String>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let shapes = match rustemon::pokemon::pokemon_shape::get_all_entries(&client).await {
        Ok(list) => list,
        Err(e) => {
            println!("error getting all shapes: {:?}", e);
            return Err(e.into());
        }
    };
    let len = shapes.len();
    let pb =
        bar.add(ProgressBar::new(len.try_into().unwrap()).with_style(crate::create_bar_style()));
    pb.finish_with_message("done");
    for (index, p) in shapes.into_iter().enumerate() {
        pb.set_message(format!("shape {}/{}", index + 1, len));
        pb.inc(1);
        let mut triples: Vec<Triple> = vec![];
        let shape_id = NamedNodeRef::new(p.url.as_str())?;
        let shape_json = match p.follow(&client).await {
            Ok(list) => list,
            Err(e) => {
                println!("error getting shape info for {}: {e}", &p.url);
                return Err(e.into());
            }
        };
        // Add rdf:type declaration
        triples.push(create_type_triple(shape_id, "PokemonShape")?);

        triples.push(Triple {
            subject: shape_id.into(),
            predicate: NamedNode::new(format!("{SCHEMA}identifier"))?,
            object: Literal::new_typed_literal(shape_json.id.to_string(), xsd::INTEGER).into(),
        });
        triples.push(Triple {
            subject: shape_id.into(),
            predicate: NamedNode::new(format!("{SCHEMA}name"))?,
            object: Literal::new_simple_literal(shape_json.name).into(),
        });

        for name in shape_json.awesome_names {
            // TODO only english for now
            if name.language.name == "en" {
                triples.push(Triple {
                    subject: shape_id.into(),
                    predicate: NamedNode::new(format!("{POKE}awsomeNames"))?,
                    object: Literal::new_simple_literal(name.awesome_name).into(),
                });
            }
        }

        for name in shape_json.names {
            // TODO only english for now
            if name.language.name == "en" {
                triples.push(Triple {
                    subject: shape_id.into(),
                    predicate: NamedNode::new(format!("{POKE}names"))?,
                    object: Literal::new_simple_literal(name.name).into(),
                });
            }
        }

        for species in shape_json.pokemon_species {
            triples.push(Triple {
                subject: shape_id.into(),
                predicate: NamedNode::new(format!("{POKE}hasPokemonSpecies"))?,
                object: NamedNodeRef::new(species.url.as_str())?.into(),
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
    async fn test_shapes() {
        assert!((shape_to_nt(
            &MultiProgress::new(),
            Arc::new(RustemonClient::default()),
            mpsc::unbounded_channel().0
        )
        .await)
            .is_ok())
    }
}
