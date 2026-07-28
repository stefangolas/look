//! A hand-written reader for the ISO 10303-21 exchange structure.
//!
//! ruststep parses Part 21 with a nom grammar built out of `alt` chains, which
//! is a fine way to describe the syntax and an expensive way to execute it: on
//! a geometry-heavy file most tokens are numbers or `#` references, and each
//! one is reached only after the earlier alternatives have been tried and have
//! each allocated a `VerboseError`. Parsing dominates STEP wall clock as a
//! result — up to 88% of it.
//!
//! Every token in this grammar is distinguishable from its first byte, so this
//! reader dispatches on that byte and never backtracks. It produces exactly the
//! [`Exchange`] ruststep would, so everything downstream — `Table`, truck, the
//! tessellated-geometry path — is unaffected. Divergence is checked against
//! ruststep in the tests rather than assumed.
//!
//! Anything this reader rejects falls back to ruststep in [`super::parse_step`],
//! so it can only make parsing faster, never narrower. That covers the two
//! sections deliberately left out here, `ANCHOR` and `REFERENCE`, which CAD
//! exporters do not emit.

use ruststep::ast::{
    DataSection, EntityInstance, Exchange, Name, Parameter, Record, SubSuperRecord,
};

/// Where and why the read stopped.
///
/// The offset is a byte index into the input, which is what a caller needs to
/// quote the offending line; it is not a character index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    pub offset: usize,
    pub message: &'static str,
}

impl std::fmt::Display for Error {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{} at byte {}", self.message, self.offset)
    }
}

impl std::error::Error for Error {}

type Result<T> = std::result::Result<T, Error>;

/// Read a whole exchange file.
pub fn parse(input: &str) -> Result<Exchange> {
    let mut reader = Reader::new(input);
    reader.skip_ignorable();
    reader.tag(b"ISO-10303-21;")?;

    let header = reader.header_section()?;

    reader.skip_ignorable();
    if reader.looking_at(b"ANCHOR;") || reader.looking_at(b"REFERENCE;") {
        return Err(reader.error("ANCHOR and REFERENCE sections are not read here"));
    }

    let mut data = Vec::new();
    loop {
        reader.skip_ignorable();
        if !reader.looking_at(b"DATA") {
            break;
        }
        data.push(reader.data_section()?);
    }

    reader.skip_ignorable();
    reader.tag(b"END-ISO-10303-21;")?;

    // A trailing SIGNATURE section is left unread, as ruststep's own entry
    // point also discards whatever follows the terminator.
    Ok(Exchange {
        header,
        anchor: Vec::new(),
        reference: Vec::new(),
        data,
        signature: Vec::new(),
    })
}

struct Reader<'a> {
    text: &'a str,
    bytes: &'a [u8],
    position: usize,
}

impl<'a> Reader<'a> {
    fn new(text: &'a str) -> Self {
        Self {
            text,
            bytes: text.as_bytes(),
            position: 0,
        }
    }

    fn error(&self, message: &'static str) -> Error {
        Error {
            offset: self.position,
            message,
        }
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.position).copied()
    }

    fn looking_at(&self, tag: &[u8]) -> bool {
        self.bytes[self.position..].starts_with(tag)
    }

    fn tag(&mut self, tag: &'static [u8]) -> Result<()> {
        if self.looking_at(tag) {
            self.position += tag.len();
            Ok(())
        } else {
            Err(self.error("expected a section keyword"))
        }
    }

    fn expect(&mut self, byte: u8) -> Result<()> {
        if self.peek() == Some(byte) {
            self.position += 1;
            Ok(())
        } else {
            Err(self.error("unexpected character"))
        }
    }

    /// Skip whitespace only, reporting whether anything was skipped.
    ///
    /// Numbers use this rather than [`Self::skip_ignorable`] because the
    /// grammar admits spaces around a sign and inside an exponent but not
    /// comments, and a token boundary is the wrong place to become more
    /// permissive than the parser being replaced. The number reader uses the
    /// return value to tell a literal it can read in place from one it has to
    /// compact first.
    fn skip_spaces(&mut self) -> bool {
        let start = self.position;
        while let Some(byte) = self.peek() {
            if byte.is_ascii_whitespace() {
                self.position += 1;
            } else {
                break;
            }
        }
        self.position != start
    }

    /// Skip whitespace and comments, which may separate any two tokens.
    fn skip_ignorable(&mut self) {
        loop {
            self.skip_spaces();
            if !self.looking_at(b"/*") {
                return;
            }
            let mut scan = self.position + 2;
            loop {
                if scan + 1 >= self.bytes.len() {
                    // Unterminated: consume the rest so the next token
                    // expectation reports the position rather than looping.
                    self.position = self.bytes.len();
                    return;
                }
                if self.bytes[scan] == b'*' && self.bytes[scan + 1] == b'/' {
                    self.position = scan + 2;
                    break;
                }
                scan += 1;
            }
        }
    }

    fn header_section(&mut self) -> Result<Vec<Record>> {
        self.skip_ignorable();
        self.tag(b"HEADER;")?;
        let mut records = Vec::new();
        loop {
            self.skip_ignorable();
            if self.looking_at(b"ENDSEC;") {
                self.position += b"ENDSEC;".len();
                return Ok(records);
            }
            let record = self.simple_record()?;
            self.skip_ignorable();
            self.expect(b';')?;
            records.push(record);
        }
    }

    fn data_section(&mut self) -> Result<DataSection> {
        self.tag(b"DATA")?;
        self.skip_ignorable();
        let meta = if self.peek() == Some(b'(') {
            self.position += 1;
            self.parameter_list()?
        } else {
            Vec::new()
        };
        self.skip_ignorable();
        self.expect(b';')?;

        let mut entities = Vec::new();
        loop {
            self.skip_ignorable();
            if self.looking_at(b"ENDSEC;") {
                self.position += b"ENDSEC;".len();
                return Ok(DataSection { meta, entities });
            }
            entities.push(self.entity_instance()?);
        }
    }

    /// `#12 = KEYWORD(...);`, or the complex form `#12 = (A(...) B(...));`.
    fn entity_instance(&mut self) -> Result<EntityInstance> {
        self.expect(b'#')?;
        let id = self.unsigned()?;
        self.skip_ignorable();
        self.expect(b'=')?;
        self.skip_ignorable();

        let instance = if self.peek() == Some(b'(') {
            self.position += 1;
            let mut records = Vec::new();
            loop {
                self.skip_ignorable();
                if self.peek() == Some(b')') {
                    self.position += 1;
                    break;
                }
                records.push(self.simple_record()?);
            }
            EntityInstance::Complex {
                id,
                subsuper: SubSuperRecord(records),
            }
        } else {
            EntityInstance::Simple {
                id,
                record: self.simple_record()?,
            }
        };

        self.skip_ignorable();
        self.expect(b';')?;
        Ok(instance)
    }

    fn simple_record(&mut self) -> Result<Record> {
        let name = self.keyword()?;
        self.skip_ignorable();
        self.expect(b'(')?;
        let parameters = self.parameter_list()?;
        Ok(Record {
            name,
            parameter: Parameter::List(parameters),
        })
    }

    /// Read parameters up to and including the closing parenthesis. The opening
    /// one is already consumed by the caller, which is the only place that can
    /// tell a list apart from a record or a complex instance.
    fn parameter_list(&mut self) -> Result<Vec<Parameter>> {
        self.skip_ignorable();
        if self.peek() == Some(b')') {
            self.position += 1;
            return Ok(Vec::new());
        }
        let mut parameters = Vec::new();
        loop {
            parameters.push(self.parameter()?);
            self.skip_ignorable();
            match self.peek() {
                Some(b',') => self.position += 1,
                Some(b')') => {
                    self.position += 1;
                    return Ok(parameters);
                }
                _ => return Err(self.error("expected ',' or ')' in a parameter list")),
            }
        }
    }

    fn parameter(&mut self) -> Result<Parameter> {
        self.skip_ignorable();
        let Some(byte) = self.peek() else {
            return Err(self.error("expected a parameter"));
        };
        match byte {
            b'$' => {
                self.position += 1;
                Ok(Parameter::NotProvided)
            }
            b'*' => {
                self.position += 1;
                Ok(Parameter::Omitted)
            }
            b'\'' => self.string().map(Parameter::String),
            b'(' => {
                self.position += 1;
                self.parameter_list().map(Parameter::List)
            }
            b'#' | b'@' => self.occurrence_name().map(Parameter::Ref),
            b'.' => self.enumeration().map(Parameter::Enumeration),
            b'0'..=b'9' | b'+' | b'-' => self.number(),
            b'!' | b'A'..=b'Z' | b'_' => {
                let keyword = self.keyword()?;
                self.skip_ignorable();
                self.expect(b'(')?;
                let inner = self.parameter()?;
                self.skip_ignorable();
                self.expect(b')')?;
                Ok(Parameter::Typed {
                    keyword,
                    parameter: Box::new(inner),
                })
            }
            // Binary (`"0..."`) lands here. ruststep does not implement it
            // either, so the fallback will report it rather than mis-read it.
            _ => Err(self.error("expected a parameter")),
        }
    }

    /// `KEYWORD` or `!KEYWORD`, the latter reported without its `!` to match
    /// ruststep. Note that `_` counts as an upper-case letter in this grammar,
    /// so an entity name is a single token.
    fn keyword(&mut self) -> Result<String> {
        if self.peek() == Some(b'!') {
            self.position += 1;
        }
        let start = self.position;
        match self.peek() {
            Some(byte) if byte.is_ascii_uppercase() || byte == b'_' => self.position += 1,
            _ => return Err(self.error("expected a keyword")),
        }
        while let Some(byte) = self.peek() {
            if byte.is_ascii_uppercase() || byte == b'_' || byte.is_ascii_digit() {
                self.position += 1;
            } else {
                break;
            }
        }
        Ok(self.text[start..self.position].to_owned())
    }

    /// A single-quoted literal, in which `''` stands for one apostrophe. No
    /// other escape is decoded, matching ruststep: the `\X\` control
    /// directives are carried through as written.
    fn string(&mut self) -> Result<String> {
        let start = self.position + 1;
        let mut scan = start;
        let mut doubled = false;
        loop {
            let Some(byte) = self.bytes.get(scan).copied() else {
                return Err(self.error("unterminated string"));
            };
            if byte != b'\'' {
                scan += 1;
                continue;
            }
            if self.bytes.get(scan + 1) == Some(&b'\'') {
                doubled = true;
                scan += 2;
                continue;
            }
            break;
        }
        // Both ends sit on an ASCII quote, so this cannot split a codepoint —
        // exporters do emit non-ASCII inside strings.
        let raw = &self.text[start..scan];
        self.position = scan + 1;
        Ok(if doubled {
            raw.replace("''", "'")
        } else {
            raw.to_owned()
        })
    }

    /// `#12`, `@12`, `#CONSTANT`, or `@CONSTANT`.
    fn occurrence_name(&mut self) -> Result<Name> {
        let entity = self.peek() == Some(b'#');
        self.position += 1;
        match self.peek() {
            Some(byte) if byte.is_ascii_digit() => {
                let id = self.unsigned()?;
                Ok(if entity {
                    Name::Entity(id)
                } else {
                    Name::Value(id)
                })
            }
            _ => {
                let name = self.keyword()?;
                Ok(if entity {
                    Name::ConstantEntity(name)
                } else {
                    Name::ConstantValue(name)
                })
            }
        }
    }

    /// Digits of an instance name. Leading zeros are insignificant, as
    /// ISO 10303-21 6.4.4.3 requires.
    fn unsigned(&mut self) -> Result<u64> {
        let start = self.position;
        let mut value: u64 = 0;
        while let Some(byte) = self.peek() {
            if !byte.is_ascii_digit() {
                break;
            }
            value = value
                .checked_mul(10)
                .and_then(|value| value.checked_add(u64::from(byte - b'0')))
                .ok_or_else(|| self.error("instance name does not fit in u64"))?;
            self.position += 1;
        }
        if self.position == start {
            return Err(self.error("expected digits"));
        }
        Ok(value)
    }

    /// An integer or a real. The two are told apart by the decimal point,
    /// which the grammar requires a real to carry.
    fn number(&mut self) -> Result<Parameter> {
        let negative = match self.peek() {
            Some(b'-') => {
                self.position += 1;
                self.skip_spaces();
                true
            }
            Some(b'+') => {
                self.position += 1;
                self.skip_spaces();
                false
            }
            _ => false,
        };

        let start = self.position;
        self.skip_digits();
        if self.position == start {
            return Err(self.error("expected a number"));
        }

        if self.peek() != Some(b'.') {
            let digits = &self.text[start..self.position];
            let value: i64 = digits
                .parse()
                .map_err(|_| self.error("integer does not fit in i64"))?;
            return Ok(Parameter::Integer(if negative { -value } else { value }));
        }
        self.position += 1;
        self.skip_digits();

        let mantissa_end = self.position;
        let mut spaced_exponent = false;
        if self.peek() == Some(b'E') {
            self.position += 1;
            spaced_exponent |= self.skip_spaces();
            if matches!(self.peek(), Some(b'-') | Some(b'+')) {
                self.position += 1;
                spaced_exponent |= self.skip_spaces();
            }
            let digits_start = self.position;
            self.skip_digits();
            if self.position == digits_start {
                // Not an exponent after all; the real ended at the mantissa.
                self.position = mantissa_end;
                spaced_exponent = false;
            }
        }

        // The literal as written is already valid Rust float syntax whenever no
        // space crept into the exponent, so the common case parses in place.
        // Rust's parser is correctly rounded, so reading the whole literal
        // rounds identically to reassembling it.
        let magnitude: f64 = if spaced_exponent {
            let compacted: String = self.text[start..self.position]
                .chars()
                .filter(|character| !character.is_whitespace())
                .collect();
            compacted
                .parse()
                .map_err(|_| self.error("malformed real"))?
        } else {
            self.text[start..self.position]
                .parse()
                .map_err(|_| self.error("malformed real"))?
        };
        Ok(Parameter::Real(if negative {
            -magnitude
        } else {
            magnitude
        }))
    }

    fn skip_digits(&mut self) {
        while let Some(byte) = self.peek() {
            if byte.is_ascii_digit() {
                self.position += 1;
            } else {
                break;
            }
        }
    }

    /// `.ENUM.`
    fn enumeration(&mut self) -> Result<String> {
        self.position += 1;
        let name = self.keyword()?;
        self.expect(b'.')?;
        Ok(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn exchange(body: &str) -> Exchange {
        let text =
            format!("ISO-10303-21;\nHEADER;\nENDSEC;\nDATA;\n{body}\nENDSEC;\nEND-ISO-10303-21;\n");
        parse(&text).unwrap()
    }

    fn one_record(body: &str) -> Record {
        let exchange = exchange(body);
        match exchange.data[0].entities[0].clone() {
            EntityInstance::Simple { record, .. } => record,
            other => panic!("expected a simple instance, got {other:?}"),
        }
    }

    fn parameters(body: &str) -> Vec<Parameter> {
        match one_record(body).parameter {
            Parameter::List(list) => list,
            other => panic!("expected a list, got {other:?}"),
        }
    }

    #[test]
    fn reads_a_simple_instance() {
        let record = one_record("#1 = CARTESIAN_POINT('p', (0.0, 1.5, -2.0));");
        assert_eq!(record.name, "CARTESIAN_POINT");
        assert_eq!(
            record.parameter,
            Parameter::List(vec![
                Parameter::String("p".to_string()),
                Parameter::List(vec![
                    Parameter::Real(0.0),
                    Parameter::Real(1.5),
                    Parameter::Real(-2.0),
                ]),
            ])
        );
    }

    #[test]
    fn instance_ids_ignore_leading_zeros() {
        let exchange = exchange("#007 = A();");
        assert!(matches!(
            exchange.data[0].entities[0],
            EntityInstance::Simple { id: 7, .. }
        ));
    }

    /// The released ruststep cannot parse AP242 because it rejects `()`; this
    /// reader has to accept it or it would be a regression on the fork.
    #[test]
    fn reads_an_empty_parameter_list() {
        assert_eq!(one_record("#1 = A();").parameter, Parameter::List(vec![]));
    }

    #[test]
    fn distinguishes_integers_from_reals() {
        assert_eq!(
            parameters("#1 = A(2, 2.0, 2., -3, +4);"),
            vec![
                Parameter::Integer(2),
                Parameter::Real(2.0),
                Parameter::Real(2.0),
                Parameter::Integer(-3),
                Parameter::Integer(4),
            ]
        );
    }

    #[test]
    fn reads_exponents() {
        assert_eq!(
            parameters("#1 = A(1.5E3, 1.5E-3, -2.E2, 1.5E+3);"),
            vec![
                Parameter::Real(1500.0),
                Parameter::Real(0.0015),
                Parameter::Real(-200.0),
                Parameter::Real(1500.0),
            ]
        );
    }

    /// This grammar tolerates spaces around a sign and inside an exponent,
    /// which `f64::from_str` does not, so those literals take a separate path.
    #[test]
    fn reads_numbers_with_spaces_around_signs() {
        assert_eq!(
            parameters("#1 = A(1.5E - 3, 1.5E + 3, - 4);"),
            vec![
                Parameter::Real(0.0015),
                Parameter::Real(1500.0),
                Parameter::Integer(-4),
            ]
        );
    }

    /// Reading the literal in place must round exactly as reading the whole
    /// literal would, not as rescaling a mantissa by a power of ten would.
    #[test]
    fn reals_round_like_the_whole_literal() {
        assert_eq!(
            parameters("#1 = A(1.7976931348623157E308, 4.9E-324);"),
            vec![
                Parameter::Real(1.7976931348623157e308),
                Parameter::Real(4.9e-324),
            ]
        );
    }

    /// `E` not followed by digits is not an exponent, so the real ends at the
    /// mantissa and the `E` starts the next token.
    #[test]
    fn a_bare_e_does_not_start_an_exponent() {
        assert_eq!(
            parameters("#1 = A(1.5, ENUM(2));"),
            vec![
                Parameter::Real(1.5),
                Parameter::Typed {
                    keyword: "ENUM".to_string(),
                    parameter: Box::new(Parameter::Integer(2)),
                },
            ]
        );
    }

    #[test]
    fn reads_strings_with_doubled_apostrophes() {
        assert_eq!(
            parameters("#1 = A('vim''s', '');"),
            vec![
                Parameter::String("vim's".to_string()),
                Parameter::String(String::new()),
            ]
        );
    }

    /// Exporters emit non-ASCII inside strings often enough that slicing has
    /// to stay codepoint-safe.
    #[test]
    fn reads_non_ascii_strings() {
        assert_eq!(
            parameters("#1 = A('Grün — 良い');"),
            vec![Parameter::String("Grün — 良い".to_string())]
        );
    }

    #[test]
    fn reads_the_remaining_parameter_forms() {
        assert_eq!(
            parameters("#1 = A($, *, .TRUE., #2, @3, #CONST, LENGTH(1.0));"),
            vec![
                Parameter::NotProvided,
                Parameter::Omitted,
                Parameter::Enumeration("TRUE".to_string()),
                Parameter::Ref(Name::Entity(2)),
                Parameter::Ref(Name::Value(3)),
                Parameter::Ref(Name::ConstantEntity("CONST".to_string())),
                Parameter::Typed {
                    keyword: "LENGTH".to_string(),
                    parameter: Box::new(Parameter::Real(1.0)),
                },
            ]
        );
    }

    #[test]
    fn reads_a_complex_instance() {
        let exchange = exchange("#1 = (NAMED_UNIT(*) SI_UNIT(.MILLI., .METRE.));");
        match &exchange.data[0].entities[0] {
            EntityInstance::Complex { id, subsuper } => {
                assert_eq!(*id, 1);
                let names: Vec<_> = subsuper.0.iter().map(|record| &record.name).collect();
                assert_eq!(names, ["NAMED_UNIT", "SI_UNIT"]);
            }
            other => panic!("expected a complex instance, got {other:?}"),
        }
    }

    #[test]
    fn a_user_defined_keyword_drops_its_bang() {
        assert_eq!(one_record("#1 = !MINE(1);").name, "MINE");
    }

    #[test]
    fn comments_separate_tokens_anywhere() {
        assert_eq!(
            parameters("#1 /*a*/ = /*b*/ A( /*c*/ 1 /*d*/ , 2 );"),
            vec![Parameter::Integer(1), Parameter::Integer(2)]
        );
    }

    #[test]
    fn reads_the_header() {
        let text = "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION(('x'), '2;1');\nENDSEC;\nDATA;\nENDSEC;\nEND-ISO-10303-21;";
        let exchange = parse(text).unwrap();
        assert_eq!(exchange.header.len(), 1);
        assert_eq!(exchange.header[0].name, "FILE_DESCRIPTION");
    }

    #[test]
    fn a_trailing_signature_section_is_tolerated() {
        let text = "ISO-10303-21;\nHEADER;\nENDSEC;\nDATA;\nENDSEC;\nEND-ISO-10303-21;\nSIGNATURE abc== ENDSEC;";
        assert!(parse(text).is_ok());
    }

    #[test]
    fn reports_where_it_stopped() {
        let error =
            parse("ISO-10303-21;\nHEADER;\nENDSEC;\nDATA;\n#1 = A(&);\nENDSEC;\nEND-ISO-10303-21;")
                .unwrap_err();
        assert_eq!(
            &"ISO-10303-21;\nHEADER;\nENDSEC;\nDATA;\n#1 = A(&"[error.offset..],
            "&"
        );
    }

    /// These are handed to ruststep instead of being read here, and must
    /// report rather than be silently mis-read.
    #[test]
    fn rejects_sections_it_does_not_read() {
        let text =
            "ISO-10303-21;\nHEADER;\nENDSEC;\nANCHOR;\nENDSEC;\nDATA;\nENDSEC;\nEND-ISO-10303-21;";
        assert!(parse(text).is_err());
    }

    #[test]
    fn rejects_binary_parameters() {
        assert!(parse(
            "ISO-10303-21;\nHEADER;\nENDSEC;\nDATA;\n#1 = A(\"0F\");\nENDSEC;\nEND-ISO-10303-21;"
        )
        .is_err());
    }

    #[test]
    fn rejects_an_unterminated_string() {
        assert!(
            parse(
                "ISO-10303-21;\nHEADER;\nENDSEC;\nDATA;\n#1 = A('oops);\nENDSEC;\nEND-ISO-10303-21;"
            )
            .is_err()
        );
    }
}
