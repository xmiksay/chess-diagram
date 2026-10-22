//! SAN tokens → legal `cozy_chess` moves. The accepted grammar mirrors
//! shakmaty 0.30's `San::from_ascii` (the previous backend) so swapping the
//! move generator doesn't change which movetext [`board_at`](super::board_at)
//! accepts or which error it reports.

use cozy_chess::{Board, File, Move, Piece, Rank, Square};

/// A syntactically valid SAN move, not yet checked against a position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum San {
    Normal {
        role: Piece,
        file: Option<File>,
        rank: Option<Rank>,
        capture: bool,
        to: Square,
        promotion: Option<Piece>,
    },
    Castle {
        king_side: bool,
    },
    /// Null moves (`--`, `Z0`) and drops (`N@f3`) are well-formed SAN but
    /// never playable in standard chess — they parse, then fail as illegal.
    Unplayable,
}

impl San {
    /// Parses one SAN token, allowing a single trailing `+` or `#`.
    pub(super) fn parse(token: &str) -> Option<San> {
        let mut reader = Reader(token.as_bytes());
        let san = reader.san()?;
        let _ = reader.eat(b'+') || reader.eat(b'#');
        reader.0.is_empty().then_some(san)
    }

    /// Resolves to the unique legal move this SAN names in `board`; `None`
    /// when no legal move matches or the match is ambiguous.
    pub(super) fn to_move(self, board: &Board) -> Option<Move> {
        // Only the named piece type can match, so skip generating the rest.
        let role = match self {
            San::Normal { role, .. } => role,
            San::Castle { .. } => Piece::King,
            San::Unplayable => return None,
        };
        let mut found = None;
        let mut matches = 0usize;
        board.generate_moves_for(board.pieces(role), |moves| {
            for mv in moves {
                if self.matches(board, moves.piece, mv) {
                    found = Some(mv);
                    matches += 1;
                }
            }
            // A second match already makes the SAN ambiguous; stop generating.
            matches > 1
        });
        found.filter(|_| matches == 1)
    }

    fn matches(self, board: &Board, piece: Piece, mv: Move) -> bool {
        // cozy-chess encodes castling as the king capturing its own rook.
        let castles = piece == Piece::King && board.colors(board.side_to_move()).has(mv.to);
        match self {
            San::Normal {
                role,
                file,
                rank,
                capture,
                to,
                promotion,
            } => {
                let captures = board.colors(!board.side_to_move()).has(mv.to)
                    || (piece == Piece::Pawn && mv.from.file() != mv.to.file());
                !castles
                    && piece == role
                    && mv.to == to
                    && file.is_none_or(|f| f == mv.from.file())
                    && rank.is_none_or(|r| r == mv.from.rank())
                    && capture == captures
                    && promotion == mv.promotion
            }
            San::Castle { king_side } => castles && (mv.to.file() > mv.from.file()) == king_side,
            San::Unplayable => false,
        }
    }
}

struct Reader<'a>(&'a [u8]);

impl Reader<'_> {
    fn peek(&self) -> Option<u8> {
        self.0.first().copied()
    }

    fn next(&mut self) -> Option<u8> {
        let (&byte, rest) = self.0.split_first()?;
        self.0 = rest;
        Some(byte)
    }

    fn eat(&mut self, byte: u8) -> bool {
        self.eat_with(|b| (b == byte).then_some(())).is_some()
    }

    /// Consumes the next byte only if `f` accepts it.
    fn eat_with<T>(&mut self, f: impl Fn(u8) -> Option<T>) -> Option<T> {
        let value = f(self.peek()?)?;
        self.next();
        Some(value)
    }

    fn square(&mut self) -> Option<Square> {
        let file = self.next().and_then(parse_file)?;
        let rank = self.next().and_then(parse_rank)?;
        Some(Square::new(file, rank))
    }

    fn san(&mut self) -> Option<San> {
        let role = match self.peek()? {
            b'O' => {
                self.next();
                if !self.eat(b'-') || !self.eat(b'O') {
                    return None;
                }
                if !self.eat(b'-') {
                    return Some(San::Castle { king_side: true });
                }
                return self.eat(b'O').then_some(San::Castle { king_side: false });
            }
            b'-' => {
                self.next();
                return self.eat(b'-').then_some(San::Unplayable);
            }
            b'Z' => {
                self.next();
                return self.eat(b'0').then_some(San::Unplayable);
            }
            b'P' | b'N' | b'B' | b'R' | b'Q' | b'K' => self.next().and_then(parse_role)?,
            _ => Piece::Pawn,
        };

        if self.eat(b'@') {
            self.square()?;
            return Some(San::Unplayable);
        }

        // A leading file/rank is either a disambiguator or, when no second
        // square follows, the destination itself.
        let file = self.eat_with(parse_file);
        let rank = self.eat_with(parse_rank);
        let (file, rank, capture, to) = if self.eat(b'x') {
            (file, rank, true, self.square()?)
        } else if let Some(to_file) = self.eat_with(parse_file) {
            let to_rank = self.next().and_then(parse_rank)?;
            (file, rank, false, Square::new(to_file, to_rank))
        } else {
            (None, None, false, Square::new(file?, rank?))
        };

        let promotion = if self.eat(b'=') {
            Some(self.next().and_then(parse_role)?)
        } else {
            None
        };

        Some(San::Normal {
            role,
            file,
            rank,
            capture,
            to,
            promotion,
        })
    }
}

fn parse_file(byte: u8) -> Option<File> {
    File::try_from(char::from(byte)).ok()
}

fn parse_rank(byte: u8) -> Option<Rank> {
    Rank::try_from(char::from(byte)).ok()
}

/// Either case, like shakmaty's `Role::from_char` (cozy-chess is lowercase-only).
fn parse_role(byte: u8) -> Option<Piece> {
    Piece::try_from(char::from(byte.to_ascii_lowercase())).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn resolve(fen: &str, san: &str) -> Option<String> {
        let board: Board = fen.parse().expect("valid FEN");
        let mv = San::parse(san).expect("valid SAN").to_move(&board)?;
        Some(mv.to_string())
    }

    const START: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

    #[test]
    fn parses_shakmaty_grammar() {
        for ok in [
            "e4", "Pe4", "exd5", "xd5", "Nf3", "Nbd7", "N1f3", "Qh4e1", "Qh4xe1", "e8=Q", "e8=q",
            "exf8=N+", "O-O", "O-O-O#", "--", "Z0", "N@f3", "Kd1#",
        ] {
            assert!(San::parse(ok).is_some(), "{ok} should parse");
        }
        for bad in [
            "", "e", "Nf", "e9", "i4", "0-0", "O-O-", "e4!", "e4+#", "e8Q", "e8=X", "nf3", "-",
            "Z", "N@", "Nf3 ",
        ] {
            assert!(San::parse(bad).is_none(), "{bad} should not parse");
        }
    }

    #[test]
    fn castling_maps_to_king_takes_rook() {
        let fen = "r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1";
        assert_eq!(resolve(fen, "O-O").as_deref(), Some("e1h1"));
        assert_eq!(resolve(fen, "O-O-O").as_deref(), Some("e1a1"));
        // The king stepping onto its own rook is castling, never a plain king move.
        assert_eq!(resolve(fen, "Kh1"), None);
        assert_eq!(resolve(fen, "Kf1").as_deref(), Some("e1f1"));
    }

    #[test]
    fn ambiguous_san_does_not_resolve() {
        let fen = "4k3/8/8/8/8/8/8/1N2KN2 w - - 0 1";
        assert_eq!(resolve(fen, "Nd2"), None);
        assert_eq!(resolve(fen, "Nbd2").as_deref(), Some("b1d2"));
        assert_eq!(resolve(fen, "Nfd2").as_deref(), Some("f1d2"));
    }

    #[test]
    fn capture_marker_must_match() {
        let fen = "4k3/8/8/3p4/4P3/8/8/4K3 w - - 0 1";
        assert_eq!(resolve(fen, "exd5").as_deref(), Some("e4d5"));
        assert_eq!(resolve(fen, "ed5"), None);
        assert_eq!(resolve(fen, "xe5"), None);
    }

    #[test]
    fn promotion_must_be_named() {
        let fen = "4k3/P7/8/8/8/8/8/4K3 w - - 0 1";
        assert_eq!(resolve(fen, "a8"), None);
        assert_eq!(resolve(fen, "a8=N").as_deref(), Some("a7a8n"));
        assert_eq!(resolve(fen, "a8=K"), None);
    }

    #[test]
    fn unplayable_san_never_resolves() {
        assert_eq!(resolve(START, "--"), None);
        assert_eq!(resolve(START, "N@f3"), None);
    }
}
