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

pub async fn flavors_to_nt(
    bar: MultiProgress,
    client: Arc<RustemonClient>,
    tx: mpsc::UnboundedSender<String>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let all_flavors = match rustemon::berries::berry_flavor::get_all_entries(&client).await {
        Ok(list) => list,
        Err(e) => {
            println!("error getting all berry flavors: {:?}", e);
            return Err(e.into());
        }
    };

    let pb = bar.add(ProgressBar::new(all_flavors.len().try_into().unwrap()));
    for (index, p) in all_flavors.into_iter().enumerate() {
        pb.set_message(format!("berry flavors #{}", index + 1));
        pb.inc(1);
        let mut triples: Vec<Triple> = vec![];
        let berry_flavor_id = NamedNodeRef::new(p.url.as_str())?;
        let berry_flavor_json = match p.follow(&client).await {
            Ok(list) => list,
            Err(e) => {
                println!("error getting berry flavor info for {}: {e}", &p.url);
                return Err(e.into());
            }
        };
        // Add rdf:type declaration
        triples.push(create_type_triple(berry_flavor_id, "BerryFlavor")?);

        triples.push(Triple {
            subject: berry_flavor_id.into(),
            predicate: NamedNode::new(format!("{SCHEMA}identifier"))?,
            object: Literal::new_typed_literal(berry_flavor_json.id.to_string(), xsd::INTEGER)
                .into(),
        });
        triples.push(Triple {
            subject: berry_flavor_id.into(),
            predicate: NamedNode::new(format!("{SCHEMA}name"))?,
            object: Literal::new_simple_literal(berry_flavor_json.name).into(),
        });
        for (i, berry) in berry_flavor_json.berries.into_iter().enumerate() {
            let flavor_to_berry_id =
                BlankNode::new(format!("flavor{}_berry{}", berry_flavor_json.id, i))?;
            triples.push(Triple {
                subject: berry_flavor_id.into(),
                predicate: NamedNode::new(format!("{POKE}hasFlavor"))?,
                object: flavor_to_berry_id.as_ref().into(),
            });
            triples.push(Triple {
                subject: flavor_to_berry_id.as_ref().into(),
                predicate: NamedNode::new(format!("{POKE}potency"))?,
                object: Literal::new_typed_literal(berry.potency.to_string(), xsd::INTEGER).into(),
            });
            triples.push(Triple {
                subject: flavor_to_berry_id.as_ref().into(),
                predicate: NamedNode::new(format!("{POKE}forBerry"))?,
                object: NamedNode::new(berry.berry.url)?.into(),
            });
        }

        triples.push(Triple {
            subject: berry_flavor_id.into(),
            predicate: NamedNode::new(format!("{POKE}contestType"))?,
            object: NamedNode::new(berry_flavor_json.contest_type.url)?.into(),
        });

        for name in berry_flavor_json.names {
            // Only include English names for now
            if name.language.name == "en" {
                triples.push(Triple {
                    subject: berry_flavor_id.into(),
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
    async fn test_flavors() {
        assert!((flavors_to_nt(
            MultiProgress::new(),
            Arc::new(RustemonClient::default()),
            mpsc::unbounded_channel().0
        )
        .await)
            .is_ok())
    }
}
