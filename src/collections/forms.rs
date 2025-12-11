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
use crate::POKEMONKG;
use crate::SCHEMA;

pub async fn form_to_nt(
    bar: &MultiProgress,
    client: Arc<RustemonClient>,
    tx: mpsc::UnboundedSender<String>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let all_forms = match rustemon::pokemon::pokemon_form::get_all_entries(&client).await {
        Ok(list) => list,
        Err(e) => {
            println!("error getting all forms: {:?}", e);
            return Err(e.into());
        }
    };
    let len = all_forms.len();
    let pb =
        bar.add(ProgressBar::new(len.try_into().unwrap()).with_style(crate::create_bar_style()));
    pb.finish_with_message("done");
    for (index, p) in all_forms.into_iter().enumerate() {
        pb.set_message(format!("form {}/{}", index + 1, len));
        pb.inc(1);
        let mut triples: Vec<Triple> = vec![];
        let form_id = NamedNodeRef::new(p.url.as_str())?;
        let form_json = match p.follow(&client).await {
            Ok(list) => list,
            Err(e) => {
                println!("error getting form info for {}: {e}", &p.url);
                return Err(e.into());
            }
        };
        // Add rdf:type declaration
        triples.push(create_type_triple(form_id, "PokemonForm")?);

        triples.push(Triple {
            subject: form_id.into(),
            predicate: NamedNode::new(format!("{SCHEMA}identifier"))?,
            object: Literal::new_typed_literal(form_json.id.to_string(), xsd::INTEGER).into(),
        });
        triples.push(Triple {
            subject: form_id.into(),
            predicate: NamedNode::new(format!("{SCHEMA}name"))?,
            object: Literal::new_simple_literal(form_json.name).into(),
        });
        triples.push(Triple {
            subject: form_id.into(),
            predicate: NamedNode::new(format!("{POKE}order"))?,
            object: Literal::new_typed_literal(form_json.order.to_string(), xsd::INTEGER).into(),
        });
        triples.push(Triple {
            subject: form_id.into(),
            predicate: NamedNode::new(format!("{POKE}formOrder"))?,
            object: Literal::new_typed_literal(form_json.form_order.to_string(), xsd::INTEGER)
                .into(),
        });
        triples.push(Triple {
            subject: form_id.into(),
            predicate: NamedNode::new(format!("{POKE}formName"))?,
            object: Literal::new_simple_literal(form_json.form_name).into(),
        });
        triples.push(Triple {
            subject: form_id.into(),
            predicate: NamedNode::new(format!("{POKE}isBattleOnly"))?,
            object: Literal::new_typed_literal(form_json.is_battle_only.to_string(), xsd::BOOLEAN)
                .into(),
        });
        triples.push(Triple {
            subject: form_id.into(),
            predicate: NamedNode::new(format!("{POKE}isDefault"))?,
            object: Literal::new_typed_literal(form_json.is_default.to_string(), xsd::BOOLEAN)
                .into(),
        });
        triples.push(Triple {
            subject: form_id.into(),
            predicate: NamedNode::new(format!("{POKE}isMega"))?,
            object: Literal::new_typed_literal(form_json.is_mega.to_string(), xsd::BOOLEAN).into(),
        });

        // pokemon
        triples.push(Triple {
            subject: form_id.into(),
            predicate: NamedNode::new(format!("{POKE}pokemon"))?,
            object: NamedNode::new(form_json.pokemon.url)?.into(),
        });

        for t in form_json.types {
            triples.push(Triple {
                subject: form_id.into(),
                predicate: NamedNode::new(format!("{POKEMONKG}hasType"))?,
                object: NamedNode::new(t.type_.url)?.into(),
            });
        }

        // TODO sprites
        if let Some(back_default) = form_json.sprites.back_default {
            // rustemon has Option<String> for sprites, while PokeAPI has image URLs
            // example: https://pokeapi.co/api/v2/pokemon-form/1/
            triples.push(Triple {
                subject: form_id.into(),
                predicate: NamedNode::new(format!("{POKE}backDefaultSprite"))?,
                object: NamedNode::new(back_default)?.into(),
            });
        }
        if let Some(front_default) = form_json.sprites.front_default {
            triples.push(Triple {
                subject: form_id.into(),
                predicate: NamedNode::new(format!("{POKE}frontDefaultSprite"))?,
                object: NamedNode::new(front_default)?.into(),
            });
        }
        if let Some(back_shiny) = form_json.sprites.back_shiny {
            triples.push(Triple {
                subject: form_id.into(),
                predicate: NamedNode::new(format!("{POKE}backShinySprite"))?,
                object: NamedNode::new(back_shiny)?.into(),
            });
        }
        if let Some(front_shiny) = form_json.sprites.front_shiny {
            triples.push(Triple {
                subject: form_id.into(),
                predicate: NamedNode::new(format!("{POKE}frontShinySprite"))?,
                object: NamedNode::new(front_shiny)?.into(),
            });
        }
        if let Some(back_shiny_female) = form_json.sprites.back_shiny_female {
            triples.push(Triple {
                subject: form_id.into(),
                predicate: NamedNode::new(format!("{POKE}backShinyFemaleSprite"))?,
                object: NamedNode::new(back_shiny_female)?.into(),
            });
        }
        if let Some(front_shiny_female) = form_json.sprites.front_shiny_female {
            triples.push(Triple {
                subject: form_id.into(),
                predicate: NamedNode::new(format!("{POKE}frontShinyFemaleSprite"))?,
                object: NamedNode::new(front_shiny_female)?.into(),
            });
        }
        if let Some(back_female) = form_json.sprites.back_female {
            triples.push(Triple {
                subject: form_id.into(),
                predicate: NamedNode::new(format!("{POKE}backFemaleSprite"))?,
                object: NamedNode::new(back_female)?.into(),
            });
        }
        if let Some(front_female) = form_json.sprites.front_female {
            triples.push(Triple {
                subject: form_id.into(),
                predicate: NamedNode::new(format!("{POKE}frontFemaleSprite"))?,
                object: NamedNode::new(front_female)?.into(),
            });
        }
        // version_group
        triples.push(Triple {
            subject: form_id.into(),
            predicate: NamedNode::new(format!("{POKE}versionGroup"))?,
            object: NamedNode::new(form_json.version_group.url)?.into(),
        });
        // names
        for n in form_json.names {
            // TODO only english for now
            if n.language.name == "en" {
                triples.push(Triple {
                    subject: form_id.into(),
                    predicate: NamedNode::new(format!("{POKE}names"))?,
                    object: Literal::new_simple_literal(n.name).into(),
                });
            }
        }
        // form_names
        for f in form_json.form_names {
            // TODO only english for now
            if f.language.name == "en" {
                triples.push(Triple {
                    subject: form_id.into(),
                    predicate: NamedNode::new(format!("{POKE}formNames"))?,
                    object: Literal::new_simple_literal(f.name).into(),
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
    async fn test_forms() {
        assert!((form_to_nt(
            &MultiProgress::new(),
            Arc::new(RustemonClient::default()),
            mpsc::unbounded_channel().0
        )
        .await)
            .is_ok())
    }
}
