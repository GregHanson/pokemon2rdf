# pokemon2rdf

**Work In Progress**

A Rust tool that converts Pokémon data from [PokéAPI](https://pokeapi.co/) into RDF (Resource Description Framework) format, generating N-Triples that align with existing Linked Data ontologies.

## Features

- **Standard Vocabulary Alignment**: Uses published ontologies instead of creating redundant terms
- **Comprehensive Coverage**: Converts abilities, moves, types, species, evolution chains, and more
- **Daily Output**: Generates timestamped RDF files (`pokemon-YYYY-MM-DD.nt`)

## Ontologies Used

This project prioritizes reusing existing vocabularies:

### Primary Ontologies

- **[PokémonKG Ontology](https://pokemonkg.org/ontology)** (`pokemonkg:`) - Domain-specific Pokémon classes and properties
  - Classes: `Species`, `Ability`, `Move`, `Type`, `Region`, `Habitat`, `EggGroup`, `Generation`, `Shape`, etc.
  - Properties: `hasType`, `evolvesFrom`, `mayHaveAbility`, `foundIn`, `inEggGroup`, `hasShape`, `accuracy`, `basePower`, etc.

- **[Schema.org](https://schema.org/)** (`schema:`) - Web standards for common metadata
  - `schema:name` - entity names
  - `schema:identifier` - IDs
  - `schema:description` - descriptions

### Fallback Namespace

- **`http://purl.org/pokemon/ontology#`** (`poke:`) - Used only for properties not covered by existing ontologies

## Current Status

### Implemented

- Abilities
- Damage Classes
- Egg Groups
- Evolution Chains
- Forms
- Generations
- Growth Rates
- Habitats
- Locations
- Moves
- Move Targets
- Natures
- Pal Park Areas
- Pokémon (including stats and moves)
- Regions
- Shapes
- Species
- Types

### TODO

- Berry Firmness
- Berries
- Berry Flavors
- Contest Types
- Contest Effects
- Encounter Methods
- Evolution Triggers
- Genders
- Items
- Item Categories
- Languages
- Move Ailments
- Move Battle Styles
- Move Categories
- Pokeathlon Stats
- Pokédex entries
- Version Groups
- Versions
- And more...

## Installation

```bash
cargo build --release
```

## Usage

```bash
cargo run --release
```

This will generate a file named `pokemon-YYYY-MM-DD.nt` in the current directory containing all the RDF triples.

## Example SPARQL Queries

Find all Pokémon that can learn Giga Drain, sorted by special attack:

```sparql
PREFIX schema: <https://schema.org/>
PREFIX poke: <http://purl.org/pokemon/ontology#>

SELECT ?pokemon ?pokemonName ?specialAttackStat
WHERE {
  ?move schema:name "giga-drain" .
  ?move poke:learnedBy ?pokemon .
  ?pokemon schema:name ?pokemonName .
  ?pokemon poke:pokemonStat ?statNode .
  ?statNode poke:stat <https://pokeapi.co/api/v2/stat/4/> .
  ?statNode poke:baseStat ?specialAttackStat .
}
ORDER BY DESC(?specialAttackStat)
```

## Data Source

Data is sourced from [PokéAPI](https://pokeapi.co/), a free and open RESTful API for Pokémon data.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

### Important Note

**Pokémon and all respective names are trademark & © of Nintendo 1996-2025, Creatures Inc., and GAME FREAK Inc.**

This project is NOT affiliated with, endorsed by, or sponsored by Nintendo, Game Freak, Creatures Inc., or The Pokémon Company. The generated RDF data is provided for educational and research purposes only.

## Contributing

This is a work in progress! Contributions are welcome, especially for:

- Adding missing PokeAPI collections
- Improving ontology alignment
- Adding SPARQL query examples
- Documentation improvements

## References

- [PokéAPI](https://pokeapi.co/)
- [PokémonKG Ontology](https://pokemonkg.org/ontology/)
- [Schema.org](https://schema.org/)
- [RDF Primer](https://www.w3.org/TR/rdf11-primer/)
- [SPARQL Query Language](https://www.w3.org/TR/sparql11-query/)
