#[test]
fn test_ack_packet_parse() {
    use rft::wire::*;
    let buf = std::fs::read("tests/data/ack_packet.bin").expect("Failed to read file");
    // TODO assert packet content equality
    // Assert that we are able to parse that packet wihtout any errors
    let _packet = Packet::parse_buf(&buf).expect("Failed to parse packet");
    let mut expected = Packet::new(69);
    expected.add_frame(Frame::Ack(AckFrame::new(1, 12)));
    let expected_buf = expected.assemble();
    assert_eq!(buf, expected_buf);
}

#[test]
fn test_ack_data_packet_parse() {
    use rft::wire::*;
    let buf = std::fs::read("tests/data/ack_data_packet.bin").expect("Failed to read file");
    // TODO assert packet content equality
    // Assert that we are able to parse that packet wihtout any errors
    let _packet = Packet::parse_buf(&buf).expect("Failed to parse packet");
    let mut expected = Packet::new(69);
    expected.add_frame(Frame::Ack(AckFrame::new(42, 3)));
    expected.add_frame(Frame::Data(DataFrame::new(
        1,
        2,
        3,
        "Did you ever hear the Tragedy of Darth Plagueis the Wise?"
            .as_bytes()
            .into(),
    )));
    let expected_buf = expected.assemble();
    assert_eq!(buf, expected_buf);
}
