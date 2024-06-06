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

## Requirements Language

{::boilerplate bcp14-tagged}

## Notation

This document defines `U8`, `U16`, `U32`, `U64` as unsigned 8-, 16-, 32-, or 64-bit integers.
A `string` is a UTF-8 {{RFC3629}} encoded zero-terminated string.

Messages are represented in a C-style notation. They may be annotated by C-style comments.
All members are laid out continuously on wire, any padding will be made explicit.

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

