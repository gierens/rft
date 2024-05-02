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

## Headers
- Connection Header
  - Connection ID, 0 for client hello, server sends back connection id
  - Next Header

- Data Header
  - Offset
  - Length
  - Flags with final bit for example
  - Checksum
  - Frame Id
  - (Next Header)

- Ack Header
  - Frame Id
  - Acknowledged cumulative range
  - Window Size for flow control
  - (Next Header)

- Error Header
  - Frame Id
  - Error code
  - Error message

- Command Header
  - Command opcode
  - Parameters
  - Frame Id
  - Next Header

## Commands
- Read
- Write
- List
- Move
- Remove
- Mkdir
- Rmdir
- Close
