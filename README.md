# rft

## Ideas
- connection id chosen by server
- use offset to reestablish connection, or on address move
- read and write commands, list, maybe move and remove, mkdir, rmdir
- list is basically a read of a virtual file
- start byte and length of data for read and write
- header chaining, so main connection header, and encapsulated headers for commands
- chaining can be used for 0-rtt read or write or commands
- checksums via fast not necessarily secure hash (md5, crc32, etc)
- preallocate file on receiver, allows parallel and out of order writes to disk
- chunk header + payload for actual data transfer
- command header also includes path, so directory structure can be maintained
- for flow control: window size in acks
- for congestion control: aimd, slow start, loss based?


Questions:
- can we move the FrameID into the general MessageType? Is it used by every packet?
- do we (not) need a general message format?

# Message Formats

## General Message Format
We need a general message format for all messages. Compare with
- Common Header defintions: https://grnvs.net.in.tum.de/cheatsheet.pdf
- SSH File Transfer Protocol: https://datatracker.ietf.org/doc/html/draft-ietf-secsh-filexfer-13#section-4
- SHH File Transfer Protocol Packet Types: https://datatracker.ietf.org/doc/html/draft-ietf-secsh-filexfer-13#section-4.3

```
Message {
  U32 Length
  U8 MessageType
  U16 ConnectionID
  U64 FrameID
  ... payload depending on MsgType
}
```
MessageType values:

0. Init // or leave free and make Init = 1?
1. Connection
2. Data
3. Ack
4. Error
6. Command

## Headers
- Connection Header
  - ConnectionID // 0: client hello, server sends back connection id
  - Next Header

- Data Header
  - Offset
  - Length
  - Flags with final bit for example
  - Checksum
  - FrameID
  - (Next Header)

- Ack Header
  - FrameID
  - Acknowledged cumulative range
  - Window Size for flow control
  - (Next Header)

- Error Header
  - FrameID
  - ErrorCode
  - ErrorMessage

- Command Header
  - CommandType
  - Parameters
  - FrameID
  - NextHeader

CommandType values:

0. // ?
1. Read
2. Write
3. List
4. Move
5. Remove
6. MkDir
7. RmDir
8. Close
