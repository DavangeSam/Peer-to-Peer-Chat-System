use crate::Bing2BingError;
use crate::Bing2BingFrame;
use bytes::Bytes;
use std::{fmt, str, vec};

/// This struct helps in the construction of commands by
/// providing convience methods for individual frame elements out of a
/// [Bing2BingFrame::Array]
#[derive(Debug)]
pub(crate) struct Parser {
    /// Array frame iterator
    parts: vec::IntoIter<Bing2BingFrame>,
}

#[derive(Debug)]
pub(crate) enum ParseError {
    /// Extracting a value failed because the frame was
    /// fully consumed.
    EndOfStream,

    /// Any other error.
    Other(Bing2BingError),
}

impl Parser {
    pub(crate) fn new(frame: Bing2BingFrame) -> Result<Parser, ParseError> {
        let array = match frame {
            Bing2BingFrame::Array(array) => array,
            frame => return Err(format!("protcol error; expected array, got {:?}", frame).into()),
        };

        Ok(Self {
            parts: array.into_iter(),
        })
    }

    /// Returns the next entry. Since [Bing2BingFrame::Array]s are arrays of frames,
    /// the next value must be a frame as well (or end of stream!)
    pub(crate) fn next(&mut self) -> Result<Bing2BingFrame, ParseError> {
        self.parts.next().ok_or(ParseError::EndOfStream)
    }

    /// Returns the next f64; if the next frame is _not_ a Float frame, an error is returned.
    pub(crate) fn next_float(&mut self) -> Result<f64, ParseError> {
        match self.next()? {
            Bing2BingFrame::Float(f) => Ok(f),
            frame => Err(format!("protocol error; expected Float frame, got {:?}", frame).into()),
        }
    }

    /// Returns the next Text frame; if the next frame is _not_ a Text frame, an error is returned.
    pub(crate) fn next_text(&mut self) -> Result<String, ParseError> {
        match self.next()? {
            Bing2BingFrame::Text(s) => Ok(s),
            frame => Err(format!("protocol error; expected Text frame, got {:?}", frame).into()),
        }
    }

    // Returns the next Text entry
    // pub(crate) fn next_string(&mut self) -> Result<String, ParseError> {
    //     match self.next()? {
    //         Bing2BingFrame::Text(s) => Ok(s),
    //         frame => Err(format!("protocol error; expected text frame, got {:?}", frame).into()),
    //     }
    // }

    pub(crate) fn next_bytes(&mut self) -> Result<Bytes, ParseError> {
        match self.next()? {
            Bing2BingFrame::Bulk(data) => Ok(Bytes::from(data)),
            frame => Err(format!("protocol error; expected bulk frame, got {:?}", frame).into()),
        }
    }

    /// Returns the next Number frame; if the next frame is _not_ a Number frame,
    /// an error is returned.
    pub(crate) fn next_number(&mut self) -> Result<u64, ParseError> {
        match self.next()? {
            Bing2BingFrame::Number(n) => Ok(n),
            frame => Err(format!("protocol error; expected Number frame, got {:?}", frame).into()),
        }
    }

    /// Returns the next Array frame, or errors otherwise.
    pub(crate) fn next_array(&mut self) -> Result<Vec<Bing2BingFrame>, ParseError> {
        match self.next()? {
            Bing2BingFrame::Array(array) => Ok(array),
            frame => Err(format!("protcol error; expected array, got {:?}", frame).into()),
        }
    }

    /// Convienence method to validate that there are no remaining frame elements
    /// to pull out of the [Bing2BingFrame::Array] that this `Parse` is reading
    /// from.
    pub(crate) fn finish(&mut self) -> Result<(), ParseError> {
        if self.parts.next().is_none() {
            Ok(())
        } else {
            Err(
                "protocol error; expected end of frame, but there was more data left in the frame"
                    .into(),
            )
        }
    }
}

impl From<String> for ParseError {
    fn from(src: String) -> Self {
        ParseError::Other(src.into())
    }
}

impl From<&str> for ParseError {
    fn from(src: &str) -> ParseError {
        src.to_string().into()
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::EndOfStream => "protocol error; unexpected end of stream".fmt(f),
            ParseError::Other(err) => err.fmt(f),
        }
    }
}

impl std::error::Error for ParseError {}
