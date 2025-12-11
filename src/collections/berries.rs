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

pub async fn berry_to_nt(
    bar: MultiProgress,
    client: Arc<RustemonClient>,
    tx: mpsc::UnboundedSender<String>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let all_berries = match rustemon::berries::berry::get_all_entries(&client).await {
        Ok(list) => list,
        Err(e) => {
            println!("error getting all berries: {:?}", e);
            return Err(e.into());
        }
    };

    let pb = bar.add(ProgressBar::new(all_berries.len().try_into().unwrap()));
    for (index, p) in all_berries.into_iter().enumerate() {
        pb.set_message(format!("berries #{}", index + 1));
        pb.inc(1);
        let mut triples: Vec<Triple> = vec![];
        let berry_id = NamedNodeRef::new(p.url.as_str())?;
        let berry_json = match p.follow(&client).await {
            Ok(list) => list,
            Err(e) => {
                println!("error getting berry info for {}: {e}", &p.url);
                return Err(e.into());
            }
        };
        // Add rdf:type declaration
        triples.push(create_type_triple(berry_id, "Berry")?);

        triples.push(Triple {
            subject: berry_id.into(),
            predicate: NamedNode::new(format!("{SCHEMA}identifier"))?,
            object: Literal::new_typed_literal(berry_json.id.to_string(), xsd::INTEGER).into(),
        });
        triples.push(Triple {
            subject: berry_id.into(),
            predicate: NamedNode::new(format!("{SCHEMA}name"))?,
            object: Literal::new_simple_literal(berry_json.name).into(),
        });
        triples.push(Triple {
            subject: berry_id.into(),
            predicate: NamedNode::new(format!("{POKE}growthTime"))?,
            object: Literal::new_typed_literal(berry_json.growth_time.to_string(), xsd::INTEGER)
                .into(),
        });
        triples.push(Triple {
            subject: berry_id.into(),
            predicate: NamedNode::new(format!("{POKE}maxHarvest"))?,
            object: Literal::new_typed_literal(berry_json.max_harvest.to_string(), xsd::INTEGER)
                .into(),
        });
        triples.push(Triple {
            subject: berry_id.into(),
            predicate: NamedNode::new(format!("{POKE}naturalGiftPower"))?,
            object: Literal::new_typed_literal(
                berry_json.natural_gift_power.to_string(),
                xsd::INTEGER,
            )
            .into(),
        });
        triples.push(Triple {
            subject: berry_id.into(),
            predicate: NamedNode::new(format!("{POKE}size"))?,
            object: Literal::new_typed_literal(berry_json.size.to_string(), xsd::INTEGER).into(),
        });
        triples.push(Triple {
            subject: berry_id.into(),
            predicate: NamedNode::new(format!("{POKE}smoothness"))?,
            object: Literal::new_typed_literal(berry_json.smoothness.to_string(), xsd::INTEGER)
                .into(),
        });
        triples.push(Triple {
            subject: berry_id.into(),
            predicate: NamedNode::new(format!("{POKE}soilDryness"))?,
            object: Literal::new_typed_literal(berry_json.soil_dryness.to_string(), xsd::INTEGER)
                .into(),
        });
        triples.push(Triple {
            subject: berry_id.into(),
            predicate: NamedNode::new(format!("{POKE}firmness"))?,
            object: NamedNode::new(berry_json.firmness.url)?.into(),
        });
        for (i, f) in berry_json.flavors.into_iter().enumerate() {
            let flavor_id = BlankNode::new(format!("berry{}_flavor{}", berry_json.id, i))?;
            triples.push(Triple {
                subject: berry_id.into(),
                predicate: NamedNode::new(format!("{POKE}hasFlavor"))?,
                object: flavor_id.as_ref().into(),
            });
            triples.push(Triple {
                subject: flavor_id.as_ref().into(),
                predicate: NamedNode::new(format!("{POKE}flavor"))?,
                object: NamedNode::new(f.flavor.url)?.into(),
            });
            triples.push(Triple {
                subject: flavor_id.as_ref().into(),
                predicate: NamedNode::new(format!("{POKE}potency"))?,
                object: Literal::new_typed_literal(f.potency.to_string(), xsd::INTEGER).into(),
            });
        }
        triples.push(Triple {
            subject: berry_id.into(),
            predicate: NamedNode::new(format!("{POKE}item"))?,
            object: NamedNode::new(berry_json.item.url)?.into(),
        });
        triples.push(Triple {
            subject: berry_id.into(),
            predicate: NamedNode::new(format!("{POKE}naturalGiftType"))?,
            object: NamedNode::new(berry_json.natural_gift_type.url)?.into(),
        });

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
    async fn test_berry() {
        assert!((berry_to_nt(
            MultiProgress::new(),
            Arc::new(RustemonClient::default()),
            mpsc::unbounded_channel().0
        )
        .await)
            .is_ok())
    }
}
