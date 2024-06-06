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
It supports connection migration based on connection IDs, in spirit similar to QUIC {{RFC9000}}, albeit a bit easier.

RFT is based on UDP, connection-oriented and stateful.
A point-to-point connection supports 

## Keywords

{::boilerplate bcp14-tagged}

## Terms

The following terms are used throughout this document:

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

The message formats are defined in a struct-like notation.

~~~~
StructName1 {
    TypeName1     FieldName1,
    TypeName2     FieldName2,
    TypeName3[4]  FieldName3,
    String        FieldName4,
    StructName2   FieldName5,
}
~~~~

The only scalar types are integer denoted with "U" for unsigned and "I" for
signed integers. Strings are a composite type consisting of the size as "U16"
followed by ASCII-characters. Padding is made explicit via the field name
"Padding" and constant values are assigned with a "=".

# Overview

This section gives a rough overview over the protocol and provides basic
information necessary to follow the detailed description in the following
sections.

The RFT protocol is a simple layer 7 protocol for Robust File Transfer.
It sits on-top of layer 4 with a single RFT packet send as a UDP SDU.
The packet structure is shown in the following figure:

~~~
                       +-----------+----------------------------------+
                       | ACK Frame |            Data Frame            |
+----------------------+-----------+----------------------------------+
| VER | CID | FN | CRC |                                              |
+----------------------+       Payload (zero or multiple frames)      |
|        Header        |                                              |
+----------------------+----------------------------------------------+
|                               RFT Packet                            |
+---------------------------------------------------------------------+
|                                UDP SDU                              |
+---------------------------------------------------------------------+
~~~

The header contains a version field (VER) for evolvability, as connection
ID (CID) uniquely identifying the connection on both ends, a frame number
(FN) counting the number of frames send in the payload, and a
cyclic-redundancy-check (CRC) checksum to validate the packet integrity.

TODO

The next section provides detailed information about connection-related
topics, e.g. establishment, reliability, congestion control and more.
The section after that explains the message format and framing in more detail,
and lists all the different frame and command types.

# Connection

The protocol is connection-based. Connections are identified a singular
connection ID unique on both sides.

## Establishment

## Teardown

## Recovery

## Migration

## Reliability

## Flow Control

## Congestion Control

## Checksumming

## Multiple Transfers

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


# IANA Considerations {#IANA}

This memo includes no request to IANA. [CHECK]


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

