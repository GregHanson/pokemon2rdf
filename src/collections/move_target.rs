use indicatif::{MultiProgress, ProgressBar};
use oxrdf::{Literal, NamedNode, NamedNodeRef, Triple};
use rustemon::client::RustemonClient;
use rustemon::Follow;
use std::error::Error;
use std::sync::Arc;
use tokio::sync::mpsc;

use crate::create_type_triple;
use crate::POKE;
use crate::SCHEMA;

pub async fn move_target_to_nt(
    bar: MultiProgress,
    client: Arc<RustemonClient>,
    tx: mpsc::UnboundedSender<String>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let all_targets = match rustemon::moves::move_target::get_all_entries(&client).await {
        Ok(list) => list,
        Err(e) => {
            println!("error getting all move targets: {:?}", e);
            return Err(e.into());
        }
    };

    let pb = bar.add(ProgressBar::new(all_targets.len().try_into().unwrap()));
    for (index, p) in all_targets.into_iter().enumerate() {
        pb.set_message(format!("move target #{}", index + 1));
        pb.inc(1);
        let mut triples: Vec<Triple> = vec![];
        let target_id = NamedNodeRef::new(p.url.as_str())?;
        let target_json = match p.follow(&client).await {
            Ok(list) => list,
            Err(e) => {
                println!("error getting move target info for {}: {e}", &p.url);
                return Err(e.into());
            }
        };
        // Add rdf:type declaration
        triples.push(create_type_triple(target_id, "MoveTarget")?);

        triples.push(Triple {
            subject: target_id.into(),
            predicate: NamedNode::new(format!("{SCHEMA}name"))?,
            object: Literal::new_simple_literal(p.name.clone()).into(),
        });
        for d in target_json.descriptions.clone() {
            if d.language.name == "en" {
                triples.push(Triple {
                    subject: target_id.into(),
                    predicate: NamedNode::new(format!("{SCHEMA}description"))?,
                    object: Literal::new_simple_literal(d.description).into(),
                });
            }
        }
        for m in target_json.moves {
            triples.push(Triple {
                subject: target_id.into(),
                predicate: NamedNode::new(format!("{POKE}move"))?,
                object: NamedNode::new(m.url)?.into(),
            });
        }
        // names
        for d in target_json.names.clone() {
            // TODO only english for now
            if d.language.name == "en" {
                triples.push(Triple {
                    subject: target_id.into(),
                    predicate: NamedNode::new(format!("{POKE}names"))?,
                    object: Literal::new_simple_literal(d.name).into(),
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
    async fn test_move_targets() {
        assert!((move_target_to_nt(
            MultiProgress::new(),
            Arc::new(RustemonClient::default()),
            mpsc::unbounded_channel().0
        )
        .await)
            .is_ok())
    }
}
