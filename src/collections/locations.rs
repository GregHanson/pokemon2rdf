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

pub async fn location_to_nt(
    bar: MultiProgress,
    client: Arc<RustemonClient>,
    tx: mpsc::UnboundedSender<String>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let all_locations = match rustemon::locations::location::get_all_entries(&client).await {
        Ok(list) => list,
        Err(e) => {
            println!("error getting all locations: {:?}", e);
            return Err(e.into());
        }
    };

    let pb = bar.add(ProgressBar::new(all_locations.len().try_into().unwrap()));
    for (index, p) in all_locations.into_iter().enumerate() {
        pb.set_message(format!("location #{}", index + 1));
        pb.inc(1);
        let mut triples: Vec<Triple> = vec![];
        let location_id = NamedNodeRef::new(p.url.as_str())?;
        let location_json = match p.follow(&client).await {
            Ok(list) => list,
            Err(e) => {
                println!("error getting location info for {}: {e}", &p.url);
                return Err(e.into());
            }
        };
        // Add rdf:type declaration
        triples.push(create_type_triple(location_id, "Location")?);

        triples.push(Triple {
            subject: location_id.into(),
            predicate: NamedNode::new(format!("{SCHEMA}identifier"))?,
            object: Literal::new_typed_literal(location_json.id.to_string(), xsd::INTEGER).into(),
        });
        triples.push(Triple {
            subject: location_id.into(),
            predicate: NamedNode::new(format!("{SCHEMA}name"))?,
            object: Literal::new_simple_literal(location_json.name).into(),
        });
        if let Some(region) = location_json.region {
            triples.push(Triple {
                subject: location_id.into(),
                predicate: NamedNode::new(format!("{POKE}region"))?,
                object: NamedNode::new(region.url)?.into(),
            });
        }
        for n in location_json.names {
            // TODO only english for now
            if n.language.name == "en" {
                triples.push(Triple {
                    subject: location_id.into(),
                    predicate: NamedNode::new(format!("{POKE}names"))?,
                    object: Literal::new_simple_literal(n.name).into(),
                });
            }
        }
        for gi in location_json.game_indices {
            let gi_id = BlankNode::default();
            triples.push(Triple {
                subject: location_id.into(),
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
        for a in location_json.areas {
            triples.push(Triple {
                subject: location_id.into(),
                predicate: NamedNode::new(format!("{SCHEMA}name"))?,
                object: NamedNode::new(a.url)?.into(),
            });
            // TODO location_area_to_nt
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
    async fn test_locations() {
        assert!((location_to_nt(
            MultiProgress::new(),
            Arc::new(RustemonClient::default()),
            mpsc::unbounded_channel().0
        )
        .await)
            .is_ok())
    }
}
