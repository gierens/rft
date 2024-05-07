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

Answer to both: no, the idea is that a single packet can hold multiple frames,
so the top-level-header just contains the connection ID and maybe the overall
length. After that follow possibly multiple and possibly different frames.

# Message Formats

## General Message Format
We need a general message format for all messages. Compare with
- Common Header defintions: https://grnvs.net.in.tum.de/cheatsheet.pdf
- SSH File Transfer Protocol: https://datatracker.ietf.org/doc/html/draft-ietf-secsh-filexfer-13#section-4
- SSH File Transfer Protocol Packet Types: https://datatracker.ietf.org/doc/html/draft-ietf-secsh-filexfer-13#section-4.3

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
```
PacketHeader (64) {
  U4 Version
  U32 ConnectionID // 0: client hello, server responds with connection id
  U8 NumberOfFrames
  U20 Checksum
  ..Frames // zero or multiple frames + payload
}
```

### Frame Headers
```
DataFrame (104) {
  U8 Type
  U32 FrameID
  U32 Offset
  U32 Length // same as SSH FTP
  (U Flags with final bit for example)
}
```
This gives us a max file size we can send of: offset * 512 bits = 2^32 * 2^9 = 2^41 bits.

```
AckFrame (40) {
  U8 Type
  U32 FrameID
}
```
```
FlowFrame (24) {
  U8 Type
  U16 WindowSize
  U8 RESERVED
}
```

```
ErrorFrame (48 + len(ErrorMessage)) {
  U8 Type
  U32 FrameID
  U8 ErrorCode
  string ErrorMessage
}
```

```
CommandFrame (48) {
  U8 Type
  U32 FrameID
  U8 CommandType
  ..CommandPayload
}
```

FrameType values:

0. // RESERVED
1. Data
2. Ack
3. Flow
4. Error
5. Command

CommandType values:

0. // RESERVED
1. Stat
2. Read
3. Write
4. List
5. Move
6. Remove
7. MkDir
8. RmDir
9. Checksum
10. Close (shouldn't this be a separate frame maybe)
