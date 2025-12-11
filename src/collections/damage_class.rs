use indicatif::{MultiProgress, ProgressBar};
use oxrdf::vocab::xsd;
use oxrdf::{Literal, NamedNode, NamedNodeRef, Triple};
use rustemon::client::RustemonClient;
use rustemon::Follow;
use std::error::Error;
use std::sync::Arc;
use tokio::sync::mpsc;

use crate::SCHEMA;
use crate::{create_bar_style, create_type_triple};

pub async fn damage_class_to_nt(
    bar: &MultiProgress,
    client: Arc<RustemonClient>,
    tx: mpsc::UnboundedSender<String>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let all_damages = match rustemon::moves::move_damage_class::get_all_entries(&client).await {
        Ok(list) => list,
        Err(e) => {
            println!("error getting all damage classes: {:?}", e);
            return Err(e.into());
        }
    };
    let len = all_damages.len();
    let pb = bar.add(ProgressBar::new(len.try_into().unwrap()).with_style(create_bar_style()));
    pb.finish_with_message("done");
    for (index, p) in all_damages.into_iter().enumerate() {
        pb.set_message(format!("move damage class {}/{}", index + 1, len));
        pb.inc(1);
        let mut triples: Vec<Triple> = vec![];
        let damage_id = NamedNodeRef::new(p.url.as_str())?;
        let damage_json = match p.follow(&client).await {
            Ok(list) => list,
            Err(e) => {
                println!("error getting damage class info for {}: {e}", &p.url);
                return Err(e.into());
            }
        };
        // Add rdf:type declaration
        triples.push(create_type_triple(damage_id, "MoveDamageClass")?);

        triples.push(Triple {
            subject: damage_id.into(),
            predicate: NamedNode::new(format!("{SCHEMA}name"))?,
            object: Literal::new_simple_literal(p.name.clone()).into(),
        });

        triples.push(Triple {
            subject: damage_id.into(),
            predicate: NamedNode::new(format!("{SCHEMA}identifier"))?,
            object: Literal::new_typed_literal(damage_json.id.to_string(), xsd::INTEGER).into(),
        });
        for d in damage_json.descriptions.clone() {
            // TODO only english for now
            if d.language.name == "en" {
                triples.push(Triple {
                    subject: damage_id.into(),
                    predicate: NamedNode::new(format!("{SCHEMA}description"))?,
                    object: Literal::new_simple_literal(d.description).into(),
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
    async fn test_damage_classes() {
        assert!((damage_class_to_nt(
            &MultiProgress::new(),
            Arc::new(RustemonClient::default()),
            mpsc::unbounded_channel().0
        )
        .await)
            .is_ok())
    }
}
