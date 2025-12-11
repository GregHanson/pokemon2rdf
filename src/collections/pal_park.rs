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

pub async fn pal_park_area_to_nt(
    bar: &MultiProgress,
    client: Arc<RustemonClient>,
    tx: mpsc::UnboundedSender<String>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let areas = match rustemon::locations::pal_park_area::get_all_entries(&client).await {
        Ok(list) => list,
        Err(e) => {
            println!("error getting all pal park areas: {:?}", e);
            return Err(e.into());
        }
    };
    let len = areas.len();
    let pb =
        bar.add(ProgressBar::new(len.try_into().unwrap()).with_style(crate::create_bar_style()));
    pb.finish_with_message("done");
    for (index, p) in areas.into_iter().enumerate() {
        pb.set_message(format!("pal park area {}/{}", index + 1, len));
        pb.inc(1);
        let mut triples: Vec<Triple> = vec![];
        let area_id = NamedNodeRef::new(p.url.as_str())?;
        let area_json = match p.follow(&client).await {
            Ok(list) => list,
            Err(e) => {
                println!("error getting pal park area info for {}: {e}", &p.url);
                return Err(e.into());
            }
        };
        // Add rdf:type declaration
        triples.push(create_type_triple(area_id, "PalParkArea")?);

        triples.push(Triple {
            subject: area_id.into(),
            predicate: NamedNode::new(format!("{SCHEMA}identifier"))?,
            object: Literal::new_typed_literal(area_json.id.to_string(), xsd::INTEGER).into(),
        });
        triples.push(Triple {
            subject: area_id.into(),
            predicate: NamedNode::new(format!("{SCHEMA}name"))?,
            object: Literal::new_simple_literal(area_json.name).into(),
        });

        for name in area_json.names {
            // TODO only english for now
            if name.language.name == "en" {
                triples.push(Triple {
                    subject: area_id.into(),
                    predicate: NamedNode::new(format!("{POKE}names"))?,
                    object: Literal::new_simple_literal(name.name).into(),
                });
            }
        }

        for (i, enc) in area_json.pokemon_encounters.into_iter().enumerate() {
            let enc_id =
                BlankNode::new(format!("palparkarea{}_pokemonencounter{}", area_json.id, i))?;
            triples.push(Triple {
                subject: area_id.into(),
                predicate: NamedNode::new(format!("{POKE}pokemonEncounters"))?,
                object: enc_id.as_ref().into(),
            });
            triples.push(Triple {
                subject: enc_id.as_ref().into(),
                predicate: NamedNode::new(format!("{POKE}baseScore"))?,
                object: Literal::new_typed_literal(enc.base_score.to_string(), xsd::INTEGER).into(),
            });
            triples.push(Triple {
                subject: enc_id.as_ref().into(),
                predicate: NamedNode::new(format!("{POKE}rate"))?,
                object: Literal::new_typed_literal(enc.rate.to_string(), xsd::INTEGER).into(),
            });
            triples.push(Triple {
                subject: enc_id.as_ref().into(),
                predicate: NamedNode::new(format!("{POKE}pokemonSpecies"))?,
                object: NamedNode::new(enc.pokemon_species.url.as_str())?.into(),
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
    async fn test_pal_park_areas() {
        assert!((pal_park_area_to_nt(
            &MultiProgress::new(),
            Arc::new(RustemonClient::default()),
            mpsc::unbounded_channel().0
        )
        .await)
            .is_ok())
    }
}
