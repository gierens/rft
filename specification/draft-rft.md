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
  RFC0768: #TCP-CCA
  RFC9000: #QUIC
  RFC3629: #UTF-8 strings
  RFC3385: #CRC32

--- abstract

Robust File Transfer (RFT) is a file-transfer protocol on top of UDP.
It is connection-oriented and stateful, supporting connection migration based on connection IDs similar to QUIC.
RFT provides point-to-point operation between a client and a server, enabling IP address migration, flow control, congestion control, and partial or resumed file transfers using offsets and lengths.

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

Empty Packet:
: A packet without frames.

Command:
: A typed request initiated by the client to the server, e.g. to initiate
a file transfer.

Sender:
: The endpoint sending a packet or frame.

Receiver:
: The endpoint receiving a packet or frame.

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

## Establishment {#establishment}

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

### Connection ID Negotiation {#connection-id-negotiation}

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
   |----------------[CID:3, FN:1][ACK, FID:15]---------------->|
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

Every packet has a checksum field containing the first 20 bits of the CRC-32
hash of the entire packet. The receiver must validate this checksum upon
receipt of a packet and discard it if if does not match and invoke a fast
retransmission.

## Migration

A connection is uniquely identified on both ends by the connection ID. As
soon as a peer receives a packet for this connection ID from a different
IP-address port pair, it must change its internal mapping and send all
subsequent packets to the new address. Any packets lost in the meantime
are subjects to retransmission. If a peer has nothing to send, but wishes
to explicitly inform the other end of a migration, the peer can simply
send an empty packet (thus a packet without frames).

## Flow Control

The congestion control is inspired TCP's AIMD algorithm.
To indicate the available receive buffer size the receiver sends back a
FlowFrame to the sender. The sender must not exceed the indicated limit
(FWND).

## Congestion Control

A congestion window (CWND) limits the amount of message in-flight.
The window scaling foes through two phases:

Slow Start:
: The congestion window starts at 1 and is updated for each packet
containing an AckFrame as "CWND_NEW = min(2 * CWND, FWND)" with FWND being
the window indicated by flow control, and ends once the slow start threshold
is reached.

Congestion Avoidance:
: After the slow start the AIMD is used. The congestion window is increased
by one for each fully acknowledged packet. In case a retransmission is
necessary congestion is assumed and the congestion window is halved and
avoidance continues from there. A timeout causes a reset of the congestion
window to one and continuous with a slow start with the threshold set half
the number of packets inflight.

# Commands

TODO write a more general section about commands, responses and error handling.

# File Transfer

File transfer works both ways via the respective command. Commands can
only be issues by the client and have to be completed before a new command
can be run.

## Read

To read a file from the server the client issues a read command with
the respective file path and indicates from which offset and how much
it wants to read. An offset of zero indicates a complete read. The server
then begins to transfer the file in chunks of data frames to the client.
The end-of-file (EOF) is signalled with an empty data frame.

The following example demonstrates the read of a file with path "hello":

~~~~ LANGUAGE-REPLACE/DELETE
Client                                                       Server
  |                                                             |
  |---[CID:3, FN:1][CMD, FID:3, READ, "hello", OFF:0, LEN:0]--->|
  |                                                             |
  |<--[CID:3, FN:2][ACK, FID:3][DATA, FID:13, OFF:0, LEN:1000]--|
  |<------[CID:3, FN:1][DATA, FID:14, OFF:1000, LEN:1000]-------|
  |<------[CID:3, FN:1][DATA, FID:15, OFF:2000, LEN:1000]-------|
  |                                                             |
  |-----------------[CID:3, FN:1][ACK, FID:15]----------------->|
  |                                                             |
  |<------[CID:3, FN:1][DATA, FID:16, OFF:3000, LEN:1000]-------|
  |<------[CID:3, FN:1][DATA, FID:17, OFF:4000, LEN:177]--------|
  |                    [DATA, FID:18, OFF:4178, LEN:0]          |
  |                                                             |
  |-----------------[CID:3, FN:1][ACK, FID:18]----------------->|
  |                                                             |
  v                                                             v
~~~~
{: title='Sequence diagram for an example file read' }

## Write

To write a file to the server the client issues a write command with
the respective file path and indicates from which offset and how much
it wants to write. An offset of zero indicates a complete read. The client
immediately transfers the file in chunks of data frames to the server.
The end-of-file (EOF) is signalled with an empty data frame.

The following example demonstrates the write of a file with path "test":

~~~~ LANGUAGE-REPLACE/DELETE
Client                                                       Server
  |                                                             |
  |---[CID:3, FN:1][CMD, FID:4, WRITE, "test", OFF:0, LEN:0]--->|
  |                [DATA, FID:5, OFF:0, LEN:1000]               |
  |-------[CID:3, FN:1][DATA, FID:6, OFF:1000, LEN:1000]------->|
  |                                                             |
  |<----------------[CID:3, FN:1][ACK, FID:6]-------------------|
  |                                                             |
  |-------[CID:3, FN:1][DATA, FID:7, OFF:2000, LEN:1000]------->|
  |-------[CID:3, FN:1][DATA, FID:8, OFF:3000, LEN:526]-------->|
  |                    [DATA, FID:9, OFF:3527, LEN:0]           |
  |                                                             |
  |-----------------[CID:3, FN:1][ACK, FID:9]------------------>|
  |                                                             |
  v                                                             v
~~~~
{: title='Sequence diagram for an example file write' }

## Multiple Transfers

The client can issue multiple commands over a single connection. Note however
that commands can only be issued once the previous command is completed or
failed. This means a connection can serve multiple transfers in sequence.

Parallelization can be achieved using multiple connections, either to
transfer different files entirely or different parts of a file using the
offset and length fields of the read and write commands.

## Recovery

When a connection drop occurs during a transfer the client may wish to
continue the transmission were it left of. For this it can re-issue the
command in a new connection with the offset set to one of the first not
received or not acknowledged frame. The optional checksum field of the
commands allow the client to ensure that the file is still the same on the
server.

# Message Formats

RFT has two types of message definitions: `Packet Header` and `Frame`s.
Messages MUST have little-endian format.
The packet header defines the top-level message, which MUST be transmitted first and defines the number of frames that follow the packet header.
The zero or multiple frames following the packer header MUST be appendend after the packer header without padding on the wire.

## Packet Header

The packet header is always the first part of a message.

* The `Version` field MUST contain the version of the protocol that is being used.
* The `ConnectionID` MUST be set to the connection ID in accordance to the connection ID negotiation upon connection establishment as described in [Connection ID Migration](#connection-id-negotiation).
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
{: title='Mandatory fields of a Packet Header.'}

## Message Frames

Multiple different frames exist.
All frames MUST start with a `U8` defining the frame type and might be followed by further fields depending on the frame.

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

The `DataFrame` frame specifies the `Offset` in bytes of the content in the file, as well as the `Length` in bytes of the data payload.
The offset and length allow for resuming interrupter transfers, partial transfers, or parallel download via multiple connections.

~~~~ language-REPLACE/DELETE
DataFrame {
  U8  Type = 0x01
  U32 FrameID
  U48 Offset
  U48 Length
}
~~~~
{: title='Mandatory fields of a Data Frame.'}


### Acknowledgment Frame

The `AckFrame` contains its frame type followed by the `FrameID` it is acknowledging.
It SHOULD use cumulative acknowledgements.

~~~~ language-REPLACE/DELETE
AckFrame {
  U8  Type = 0x02
  U32 FrameID
}
~~~~
{: title='Mandatory fields of a Acknowledgment Frame.'}

### Flow Frame

The `FlowFrame` notifies its communication partner its available `WindowsSize` in bytes.
The receiver sends the FlowFrame to the sender.
A sender SHOULD take care to never exceed this limit.
If the window remains zero for five consecutive messages, the sender MUST assume the the recevier has failed and terminate the stream.

~~~~ language-REPLACE/DELETE
FlowFrame {
  U8  Type = 0x03
  U16 WindowSize
  U8  RESERVED
}
~~~~
{: title='Mandatory fields of a Flow Frame.'}

### Error Frame

The `ErrorFrame` is used to signal an error in the transfer logic of an error that occured when executing a command specified by a `CommandFrame`.
The `ErrorCode` defines the error code and the `ErrorMessage` an optional error message, otherwise it SHOULD be empty ("").

~~~~ language-REPLACE/DELETE
ErrorFrame {
  U8  Type = 0x04
  U32 FrameID
  U8  ErrorCode
  Str ErrorMessage
}
~~~~
{: title='Mandatory fields of a Error Frame.'}

### Connection ID Change Frame

This frame MUST only be used during the initial connection ID negotiation.
As described in [Establishment](#establishment), this frame is only relevant when the client wishes to establish mutliple connections over a single port.
It MUST attach a suggested ConnectionIDChangeFrame with a proposed connection ID.
If that ID is already used, the server MUST respond with a new unique ID chosen by the server itself.

~~~~ language-REPLACE/DELETE
ConnectionIDChangeFrame {
  U8  Type = 0x05
  U32 FrameID
  U32 OldConnectionID
  U32 NewConnectionID
}
~~~~
{: title='Mandatory fields of a Connection ID Change Frame.'}

### Command Frames

The `CommandFrame` specifies the command sent by a client, based on the `CommandType`.

~~~~ language-REPLACE/DELETE
CommandFrame {
  U8  Type = 0x06
  U32 FrameID
  U8  CommandType
  // might be followed by a command payload
}
~~~~
{: title='Mandatory fields of a Command Frame.'}

The `AnswerFrame` is the response frame to a command, where the `CommandType` MUST be identical to the `CommandType` it is responding to.

~~~~ language-REPLACE/DELETE
AnswerFrame {
  U8  Type = 0x07
  U32 FrameID
  U8  CommandType
  // might be followed by a answer payload
}
~~~~
{: title='Mandatory fields of a Answer Frame.'}

| Command Type Value | Command Type       |
| 0                  | Currently reserved |
| 1                  | Read               |
| 2                  | Write              |
| 3                  | List               |
| 4                  | Delete             |
| 5                  | Stat               |
| 6                  | Exit               |
{: title="Command type definitions."}

The `ReadCmdPayload` is used to initiate the transfer of a file from the server to the client, whereas the content is specified by the `Offset` and `Length` field, which MUST be in bytes.
The `Path` is a string encoded path to the to be transferred file.
The `Checksum` MUST be the CRC-32 checksum of the file to be transferred.
The checksum value can be used to indicate if the content has changed, should the previous transcation be interrupted and the client is trying to resume the transfer at the specified offset.

~~~~ language-REPLACE/DELETE
ReadCmdPayload {
  U48 Offset
  U48 Length
  U32 Checksum
  Str Path
}
~~~~
{: title='Mandatory fields of a Read Command Payload Frame.'}

--- back
