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

pub async fn region_to_nt(
    bar: &MultiProgress,
    client: Arc<RustemonClient>,
    tx: mpsc::UnboundedSender<String>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let all_regions = match rustemon::locations::region::get_all_entries(&client).await {
        Ok(list) => list,
        Err(e) => {
            println!("error getting all regions: {:?}", e);
            return Err(e.into());
        }
    };
    let len = all_regions.len();
    let pb =
        bar.add(ProgressBar::new(len.try_into().unwrap()).with_style(crate::create_bar_style()));
    pb.finish_with_message("done");
    for (index, p) in all_regions.into_iter().enumerate() {
        pb.set_message(format!("region {}/{}", index + 1, len));
        pb.inc(1);
        let mut triples: Vec<Triple> = vec![];
        let region_id = NamedNodeRef::new(p.url.as_str())?;
        let region_json = match p.follow(&client).await {
            Ok(list) => list,
            Err(e) => {
                println!("error getting region info for {}: {e}", &p.url);
                return Err(e.into());
            }
        };

        // Add rdf:type declaration
        triples.push(create_type_triple(region_id, "Region")?);

        triples.push(Triple {
            subject: region_id.into(),
            predicate: NamedNode::new(format!("{SCHEMA}identifier"))?,
            object: Literal::new_typed_literal(region_json.id.to_string(), xsd::INTEGER).into(),
        });
        triples.push(Triple {
            subject: region_id.into(),
            predicate: NamedNode::new(format!("{SCHEMA}name"))?,
            object: Literal::new_simple_literal(region_json.name).into(),
        });
        for n in region_json.names {
            // TODO only english for now
            if n.language.name == "en" {
                triples.push(Triple {
                    subject: region_id.into(),
                    predicate: NamedNode::new(format!("{POKE}names"))?,
                    object: Literal::new_simple_literal(n.name).into(),
                });
            }
        }
        for l in region_json.locations {
            triples.push(Triple {
                subject: region_id.into(),
                predicate: NamedNode::new(format!("{POKE}hasLocation"))?,
                object: NamedNode::new(&l.url)?.into(),
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
    async fn test_regions() {
        assert!((region_to_nt(
            &MultiProgress::new(),
            Arc::new(RustemonClient::default()),
            mpsc::unbounded_channel().0
        )
        .await)
            .is_ok())
    }
}
