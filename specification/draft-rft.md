---
title: Robust File Transfer based on Simplified QUIC for File System Access
abbrev: RFT
docname: draft-rft
date: 2024-06-01
lang: en

ipr: trust200902
cat: info # Check
submissiontype: IETF
area: Applications
wg: TUM Protocol Design
kw: Internet-Draft
#stand_alone: true
#ipr: trust200902
#cat: info # Check
#submissiontype: IETF
#area: General [REPLACE]
#wg: Internet Engineering Task Force


#obsoletes: 4711, 4712 # Remove if not needed/Replace
#updates: 4710 # Remove if not needed/Replace


# date: 2022-02-02 -- date is filled in automatically by xml2rfc if not given
author:
- role: editor # remove if not true
  ins: N. Stangl
  name: Niklas Stangl
  org: Technical University of Munich
  street: Boltzmannstraße 3
  city: Garching
  code: 85748
  country: DE # use TLD (except UK) or country name
  email: niklas.stangl@tum.de
- role: editor # remove if not true
  ins: J. Pfannschmidt
  name: Johannes Pfannschmidt
  org: Technical University of Munich
  street: Boltzmannstraße 3
  city: Garching
  code: 85748
  country: DE # use TLD (except UK) or country name
  email: johannes.pfannschmidt@cs.tum.edu
- role: editor # remove if not true
  ins: S. Gierens
  name: Sandro-Alessio Gierens
  org: Technical University of Munich
  street: Boltzmannstraße 3
  city: Garching
  code: 85748
  country: DE # use TLD (except UK) or country name
  email: sandro.gierens@tum.de

normative:
  RFC0768: #UDP
  RFC9000: #QUIC
  RFC3629: #UTF-8 strings
  RFC3385: #CRC32
  RFC5234: #TODO: remove me later
informative:
  exampleRefMin:
    title: Title [REPLACE]
    author:
    - name: Givenname Surname[REPLACE]
      org: (ignored here anyway)
    - name: Givenname Surname1 Surname2
      surname: Surname1 Surname2 # needed for Spanish names etc.
      org: (ignored here anyway)
    date: 2006
  exampleRefOrg:
    target: http://www.example.com/
    title: Title [REPLACE]
    author:
    - org: Organization [REPLACE]
    date: 1984-04

--- abstract

Robust File Transfer (RFT) is a file-transfer protocol on top of UDP.
RFT ist based on UDP datagram transports

--- middle

# Introduction

The Protocol Design WG is tasked with standardizing an Application Protocol for a robust file transfer protocol, RFT.
This protocol is intended to provide point-to-point operation between a client and a server built upon UDP {{RFC0768}}.
It supports connection migration based on connection IDs, in spirit similar to QUIC {{RFC9000}}, although a bit easier.

RFT is based on UDP, connection-oriented and stateful.
A point-to-point connection supports IP address migration, flow control, congestion control and allows to transfers of a specific length and offset, which can be useful to resume interrupted transfers or partial transfers.
The protocol guarantees in-order delivery for all packets belonging to a stream.
There is no such guarantee for messages belonging to different streams.

RFT *messages* always consist of a single *Packet Header* and zero or multiple *Frames* appended continously on the wire after the packet header without padding.
Frames are either *data frames*, *error frames* or various types of control frames used for the connection initialization and negotiation, flow control, congestion control, acknowledgement or handling of commands.

## Keywords

{::boilerplate bcp14-tagged}

## Terms

The following terms are used throughout this document:

{:vspace}
Client:
: The endpoint of a connection that initiated it and issues commands over it.

Server:
: The endpoint of a connection that listens for and accepts connections
from clients and answers their commands.

Connection:
: A communication channel between a client and server identified by a
single connection ID unique on both ends.

Packet:
: An RFT datagram send as UDP SDU over a connection containing zero or multiple
frames.

Frame:
: A typed and sized information unit making up (possible with others) the
payload of an RFT packet.

Command:
: A typed request initiated by the client to the server, e.g. to initiate
a file transfer.

## Notation

This document defines `U4`, `U8`, `U16`, `U32`, `U64` as unsigned 4-, 8-, 16-, 32-, or 64-bit integers.
A `string` is a UTF-8 {{RFC3629}} encoded zero-terminated string.

Messages are represented in a C struct-like notation. They may be annotated by C-style comments.
All members are laid out continuously on wire, any padding will be made explicit.
Constant values are assigned with a "=".

~~~~
StructName1 {
    TypeName1     FieldName1,
    TypeName2     FieldName2,
    TypeName3[4]  FieldName3,
    String        FieldName4,
    StructName2   FieldName5,
}
~~~~

# Body [REPLACE]

Some body text [REPLACE]

This document normatively references {{RFC5234}} and has more
information in {{exampleRefMin}} and {{exampleRefOrg}}. [REPLACE]

1. Ordered list item [REPLACE/DELETE]
2. Ordered list item [REPLACE/DELETE]

* Bulleted list item [REPLACE/DELETE]
* Bulleted list item [REPLACE/DELETE]


{:vspace}
First term:
: Definition of the first term

Second term:
: Definition of the second term
<!-- Omit the leading {:vspace} for a compact definition list,
     i.e., to start definitions on same line as the term -->


| Table head 1 [REPLACE] | Table head2 [REPLACE] |
| Cell 11 [REPLACE]      | Cell 12 [REPLACE]     |
| Cell 21 [REPLACE]      | Cell 22 [REPLACE]     |
{: title="A nice table [REPLACE]"}

~~~~ language-REPLACE/DELETE
source code goes here [REPLACE]
~~~~
{: title='Source [REPLACE]' sourcecode-markers="true"}

# Message Formats

RFT has two types of message definitions: `Packet Header` and `Frame`s.
Messages MUST have little-endian format.
The packet header defines the top-level message, which MUST be transmitted first and defines the number of frames that follow the packet header.
The zero or multiple frames following the packer header MUST be appendend after the packer header without padding on the wire.

## Packet Header

The packet header is always the first part of a message.

* The `Version` field MUST contain the version of the protocol that is being used.
* The `ConnectionID` MUST be set to
* The `NumberOfFrames` field MUST be set to the number of frames that are appended after this packet header and belong to it.
* The `Checksum` field contains 20-bit of the CRC-32 hash {{RFC3385}} of the entire message, inlcuding the packet header and all of its appended frames and thei potential payload. It MUST take the first 20-bit of the 32-bit hash.

~~~~ language-REPLACE/DELETE
PacketHeader {
  U4  Version
  U32 ConnectionID   // 0: client hello, server responds with connection id
  U8  NumberOfFrames // zero or more frames + payload
  U20 Checksum
  // Zero or more appended frames
}
~~~~
{: title='Mandatory fields of a Packet Header.' sourcecode-markers="true"}

## Message Frames

Multiple different frames exist.
All frames MUST start with a `U8` defining the frame type.

| Frame Type Value | Frame Type                 |
| 0                | Currently reserved         |
| 1                | Data Frame                 |
| 2                | Acknowledgement Frame      |
| 3                | Flow Frame                 |
| 4                | Error Frame                |
| 5                | Connection ID Change Frame |
| 6                | Command Frame              |
| 7                | Answer Frame               |
| 8                | Read Command Payload Frame |
{: title="Frame type definitions."}

### Data Frame

The `DataFrame` frame contains the 

~~~~ language-REPLACE/DELETE
DataFrame {
  U8  Type
  U32 FrameID
  U48 Offset
  U48 Length
}
~~~~
{: title='Mandatory fields of a Data Frame.' sourcecode-markers="true"}


### Acknowledgment Frame

The `AckFrame` contains its frame type followed by the `FrameID` it is acknowledging.

~~~~ language-REPLACE/DELETE
AckFrame {
  U8  Type
  U32 FrameID
}
~~~~
{: title='Mandatory fields of a Acknowledgment Frame.' sourcecode-markers="true"}

### Flow Frame

~~~~ language-REPLACE/DELETE
FlowFrame {
  U8  Type
  U16 WindowSize
  U8  RESERVED
}
~~~~
{: title='Mandatory fields of a Flow Frame.' sourcecode-markers="true"}

### Error Frame

The `ErrorFrame` is used to signal an error in the transfer logic of an error that occured when executing a command specified by a `CommandFrame`.
The `ErrorCode` defines the error code and the `ErrorMessage` an optional error message.

~~~~ language-REPLACE/DELETE
ErrorFrame {
  U8  Type
  U32 FrameID
  U8  ErrorCode
  Str ErrorMessage
}
~~~~
{: title='Mandatory fields of a Error Frame.' sourcecode-markers="true"}

### Connection ID Change Frame

~~~~ language-REPLACE/DELETE
ConnectionIDChangeFrame {
  U8  Type
  U32 FrameID
  U32 OldConnectionID
  U32 NewConnectionID
}
~~~~
{: title='Mandatory fields of a Connection ID Change Frame.' sourcecode-markers="true"}

### Command Frames

~~~~ language-REPLACE/DELETE
CommandFrame {
  U8  Type
  U32 FrameID
  U8  CommandType
  // ..CommandPayload
}
~~~~
{: title='Mandatory fields of a Command Frame.' sourcecode-markers="true"}

~~~~ language-REPLACE/DELETE
AnswerFrame {
  U8  Type
  U32 FrameID
  U8  CommandType
  // ..AnswerPayload
}
~~~~
{: title='Mandatory fields of a Answer Frame.' sourcecode-markers="true"}

~~~~ language-REPLACE/DELETE
ReadCmdPayload {
  U48 Offset
  U48 Length
  U32 Checksum //changed on server?
  Str Path
}
~~~~
{: title='Mandatory fields of a Read Command Payload Frame.' sourcecode-markers="true"}

# Security Considerations {#Security}

This document should not affect the security of the Internet. [CHECK]


--- back

# Appendix 1 [REPLACE/DELETE]

This becomes an Appendix [REPLACE]


# Acknowledgements {#Acknowledgements}
{: numbered="false"}

This template uses extracts from templates written by
{{{Pekka Savola}}}, {{{Elwyn Davies}}} and
{{{Henrik Levkowetz}}}. [REPLACE]

