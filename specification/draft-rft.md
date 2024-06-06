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

~~~~ LANGUAGE-REPLACE/DELETE
StructName1 (Length) {
    TypeName1     FieldName1,
    TypeName2     FieldName2,
    TypeName3[4]  FieldName3,
    String        FieldName4,
    StructName2   FieldName5,
}
~~~~
{: title='Message format notation' }

The only scalar types are integer denoted with "U" for unsigned and "I" for
signed integers. Strings are a composite type consisting of the size as "U16"
followed by ASCII-characters. Padding is made explicit via the field name
"Padding" and constant values are assigned with a "=".

To visualize protocol runs we use the following sequence diagram notation:

~~~~ LANGUAGE-REPLACE/DELETE
Client                                                       Server
   |                                                           |
   |-------[CID:1337, FN:2][ACK, FID:3][FLOW, SIZE:1000]------>|
   |                                                           |
   v                                                           v
~~~~
{: title='Sequence diagram notation' }

The individual parts of the packets are enclosed by brackets and only the
relevant values are shown. First we always have the RFT packet header,
followed by zero or multiple frames. See below for more details on the
packet structure.

# Overview

This section gives a rough overview over the protocol and provides basic
information necessary to follow the detailed description in the following
sections.

The RFT protocol is a simple layer 7 protocol for Robust File Transfer.
It sits on-top of layer 4 with a single RFT packet send as a UDP SDU.
The packet structure is shown in the following figure:

~~~~ LANGUAGE-REPLACE/DELETE
                       +-----------+--------------------------------+
                       | ACK Frame |       Data Frame       |  ...  |
+----------------------+-----------+--------------------------------+
| VER | CID | FN | CRC |                                            |
+----------------------+      Payload (zero or multiple frames)     |
|        Header        |                                            |
+----------------------+--------------------------------------------+
|                               RFT Packet                          |
+-------------------------------------------------------------------+
|                                UDP SDU                            |
+-------------------------------------------------------------------+
~~~~
{: title='General packet structure' }

The header contains a version field (VER) for evolvability, as connection
ID (CID) uniquely identifying the connection on both ends, a frame number
(FN) counting the number of frames send in the payload, and a
cyclic-redundancy-check (CRC) checksum to validate the packet integrity.

After the header follows the payload which holds one or more RFT frames
inspired by {{RFC9000}}. These serve both for data transfer as well as any
additional logic besides version matching, connection identification, and
packet integrity validation. The most important types are AckFrames for
acknowledging frames based on their frame ID (FID), CommandFrames to issue
commands on the server, and DataFrames to transport data for the commands to
read or write a file. File data in the ReadCommand and WriteCommand as well
as in DataFrames is indexed by byte offset and length making both transfer
recovery and parallel transfers even of different parts of the same file
possible.

The next section provides detailed information about connection-related
topics, e.g. establishment, reliability, congestion control and more.
The section after that explains the message format and framing in more detail,
and lists all the different frame and command types.

# Connection

The protocol is connection-based. Connections are identified a singular
connection ID (CID) unique on both sides.

## Establishment

The connection establishment is and via a two-way handshake and is initiated by
the client by sending a packet with connection ID 0. The server responds with
the UDP packet having reversed IP addresses and ports, containing an RFT
packet with the connection ID chosen by the server. The server knows all
IDs of established connections and must make the new one is unique.

~~~~ LANGUAGE-REPLACE/DELETE
Client                                                       Server
   |                                                           |
   |----------------------[CID:0, FN:0]----------------------->|
   |                                                           |
   |<---------------------[CID:1, FN:0]------------------------|
   |                                                           |
   v                                                           v
~~~~
{: title='Sequence diagram of simple connection establishment' }

### Connection ID Negotiation

This simple connection establishment is limited to a single handshake
at a time per UDP source port. If the client wishes to establish multiple over
a single port it can attach a ConnectionIdChangeFrame with a proposed
connection ID for the new one (NEW) and 0 for the old one (OLD). The server
acknowledges this and sends back the handshake response to that connection ID:

~~~~ LANGUAGE-REPLACE/DELETE
Client                                                       Server
   |                                                           |
   |--------[CID:0, FN:2][CHCID, FID:1, OLD:0, NEW:3]--------->|
   |                                                           |
   |<----------------[CID:3, FN:0][ACK, FID:1]-----------------|
   |                                                           |
   v                                                           v
~~~~
{: title='Sequence diagram of successful connection ID proposal' }

In case the proposal is already used for another connection
attaches another ConnectionIdChangeFrame (CHCID) with the new unique connection
ID chosen by the server.

~~~~ LANGUAGE-REPLACE/DELETE
Client                                                       Server
   |                                                           |
   |--------[CID:0, FN:1][CHCID, FID:1, OLD:0, NEW:3]--------->|
   |                                                           |
   |<--[CID:3, FN:2][ACK, FID:1][CHCID, FID:1, OLD:3, NEW:9]---|
   |                                                           |
   |-----------------[CID:9, FN:0][ACK, FID:1]---------------->|
   |                                                           |
   v                                                           v
~~~~
{: title='Sequence diagram of unsuccessful connection ID proposal' }

### Version Interoperability

Before responding to a handshake response the server must validate that the
client protocol version is interoperable with its own. So long as RFT is
still in draft phase with rapid breaking changes the versions of client
and server have to strictly match.

## Teardown

If the client wishes to close the connection it simply sends a ExitCommand.
Then the AckFrame for this command is the last one the server sends for this
connection.

~~~~ LANGUAGE-REPLACE/DELETE
Client                                                       Server
   |                                                           |
   |------------[CID:5, FN:1][CMD, FID:1234, EXIT]------------>|
   |                                                           |
   |<--------------[CID:5, FN:1][ACK, FID:1234]----------------|
   |                                                           |
   v                                                           v
~~~~
{: title='General packet structure' }

## Reliability

The protocol achieves realiability by acknowledgements and checksumming.

### Frame ID

Most frame types carry a frame ID. This is basically the count of frames
the endpoint sending the frame has sent so far, so it starts at 1 and
is incremented by 1 for each frame sent. A wrap around occurs when the
maximum value is reached.

### Acknowledgement

Frames are cumulatively acknowledged by the receiver. The receiver sends
an AckFrame with the frame ID of the last frame it received. The sender
then knows that all frames up to this frame ID have been received.

~~~~ LANGUAGE-REPLACE/DELETE
Client                                                       Server
   |                                                           |
   |<-------[CID:3, FN:1][DATA, FID:13, OFF:0, LEN:1000]-------|
   |<-----[CID:3, FN:1][DATA, FID:14, OFF:1000, LEN:1000]------|
   |<-----[CID:3, FN:1][DATA, FID:15, OFF:2000, LEN:1000]------|
   |                                                           |
   |----------------[CID:3, FN:0][ACK, FID:15]---------------->|
   |                                                           |
   v                                                           v
~~~~
{: title='Sequence diagram of frame cumulative acknowledgement' }

### Retransmission

If the sender does not receive an AckFrame for a frame it sent within a
timeout 5 seconds it retransmits the frame. If the receiver misses a previous
frame it sends a duplicate AckFrame for the previous frame ID to signal the
sender to do a fast retransmission.

### Checksumming

## Recovery

## Migration

## Flow Control

## Congestion Control

## Multiple Transfers

## Timeout

# File Transfer

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

~~~~ language-REPLACE/DELET
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

