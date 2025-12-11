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

pub async fn growth_rate_to_nt(
    bar: &MultiProgress,
    client: Arc<RustemonClient>,
    tx: mpsc::UnboundedSender<String>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let all_rates = match rustemon::pokemon::growth_rate::get_all_entries(&client).await {
        Ok(list) => list,
        Err(e) => {
            println!("error getting all growth rates: {:?}", e);
            return Err(e.into());
        }
    };
    let len = all_rates.len();
    let pb =
        bar.add(ProgressBar::new(len.try_into().unwrap()).with_style(crate::create_bar_style()));
    pb.finish_with_message("done");
    for (index, p) in all_rates.into_iter().enumerate() {
        pb.set_message(format!("growth rate {}/{}", index + 1, len));
        pb.inc(1);
        let mut triples: Vec<Triple> = vec![];
        let growth_id = NamedNodeRef::new(p.url.as_str())?;
        let growth_json = match p.follow(&client).await {
            Ok(list) => list,
            Err(e) => {
                println!("error getting growth rate: {:?}", e);
                return Err(e.into());
            }
        };
        // Add rdf:type declaration
        triples.push(create_type_triple(growth_id, "GrowthRate")?);

        triples.push(Triple {
            subject: growth_id.into(),
            predicate: NamedNode::new(format!("{SCHEMA}identifier"))?,
            object: Literal::new_typed_literal(growth_json.id.to_string(), xsd::INTEGER).into(),
        });
        triples.push(Triple {
            subject: growth_id.into(),
            predicate: NamedNode::new(format!("{SCHEMA}name"))?,
            object: Literal::new_simple_literal(growth_json.name).into(),
        });
        triples.push(Triple {
            subject: growth_id.into(),
            predicate: NamedNode::new(format!("{POKE}formula"))?,
            object: Literal::new_simple_literal(growth_json.formula).into(),
        });
        for d in growth_json.descriptions {
            // TODO only English for now
            if d.language.name == "en" {
                triples.push(Triple {
                    subject: growth_id.into(),
                    predicate: NamedNode::new(format!("{SCHEMA}description"))?,
                    object: Literal::new_simple_literal(d.description).into(),
                });
            }
        }
        for l in growth_json.levels {
            let level_id =
                BlankNode::new(format!("growthrate{}_explevel{}", growth_json.id, l.level))?;
            triples.push(Triple {
                subject: growth_id.into(),
                predicate: NamedNode::new(format!("{POKE}hasExpLevel"))?,
                object: level_id.as_ref().into(),
            });
            triples.push(Triple {
                subject: level_id.as_ref().into(),
                predicate: NamedNode::new(format!("{POKE}level"))?,
                object: Literal::new_typed_literal(l.level.to_string(), xsd::INTEGER).into(),
            });
            triples.push(Triple {
                subject: level_id.as_ref().into(),
                predicate: NamedNode::new(format!("{POKE}experience"))?,
                object: Literal::new_typed_literal(l.experience.to_string(), xsd::INTEGER).into(),
            });
        }
        // pokemon_species
        for p in growth_json.pokemon_species {
            triples.push(Triple {
                subject: growth_id.into(),
                predicate: NamedNode::new(format!("{POKE}hasSpecies"))?,
                object: NamedNode::new(p.url)?.into(),
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
    async fn test_growth_rates() {
        assert!((growth_rate_to_nt(
            &MultiProgress::new(),
            Arc::new(RustemonClient::default()),
            mpsc::unbounded_channel().0
        )
        .await)
            .is_ok())
    }
}
