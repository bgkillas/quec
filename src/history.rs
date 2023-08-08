pub struct History
{
    pub pos: usize,
    pub list: Vec<Point>,
}
pub struct Point
{
    pub add: bool,
    pub split: bool,
    pub pos: (usize, usize),
    pub char: char,
    pub line: Option<Vec<char>>,
}
//stolen from chatgpt, probably is innefficient
impl History
{
    pub(crate) fn to_bytes(&self) -> Vec<u8>
    {
        let mut bytes = Vec::new();
        bytes.extend(&self.pos.to_le_bytes());
        bytes.extend(&self.list.len().to_le_bytes());
        for point in &self.list
        {
            let point_bytes = point.to_bytes();
            bytes.extend(&point_bytes.len().to_le_bytes());
            bytes.extend(point_bytes);
        }
        bytes
    }
    pub(crate) fn from_bytes(bytes: &[u8]) -> History
    {
        let mut cursor = 0;
        let pos = usize::from_le_bytes([
            bytes[cursor],
            bytes[cursor + 1],
            bytes[cursor + 2],
            bytes[cursor + 3],
            bytes[cursor + 4],
            bytes[cursor + 5],
            bytes[cursor + 6],
            bytes[cursor + 7],
        ]);
        cursor += 8;
        let list_len = usize::from_le_bytes([
            bytes[cursor],
            bytes[cursor + 1],
            bytes[cursor + 2],
            bytes[cursor + 3],
            bytes[cursor + 4],
            bytes[cursor + 5],
            bytes[cursor + 6],
            bytes[cursor + 7],
        ]);
        cursor += 8;
        let mut list = Vec::with_capacity(list_len);
        for _ in 0..list_len
        {
            let point_size = usize::from_le_bytes([
                bytes[cursor],
                bytes[cursor + 1],
                bytes[cursor + 2],
                bytes[cursor + 3],
                bytes[cursor + 4],
                bytes[cursor + 5],
                bytes[cursor + 6],
                bytes[cursor + 7],
            ]);
            cursor += 8;
            let point_bytes = &bytes[cursor..cursor + point_size];
            list.push(Point::from_bytes(point_bytes));
            cursor += point_size;
        }
        History { pos, list }
    }
}
impl Point
{
    fn to_bytes(&self) -> Vec<u8>
    {
        let mut bytes = Vec::new();
        bytes.extend(&[self.add as u8, self.split as u8]);
        bytes.extend(&self.pos.0.to_le_bytes());
        bytes.extend(&self.pos.1.to_le_bytes());
        bytes.push(self.char as u8);
        match &self.line
        {
            Some(line) =>
            {
                bytes.push(1);
                bytes.extend(&line.len().to_le_bytes());
                bytes.extend(line.iter().map(|&c| c as u8));
            }
            None =>
            {
                bytes.push(0);
            }
        }
        bytes
    }
    fn from_bytes(bytes: &[u8]) -> Point
    {
        let mut cursor = 0;
        let add = bytes[cursor] != 0;
        cursor += 1;
        let split = bytes[cursor] != 0;
        cursor += 1;
        let pos_0 = usize::from_le_bytes([
            bytes[cursor],
            bytes[cursor + 1],
            bytes[cursor + 2],
            bytes[cursor + 3],
            bytes[cursor + 4],
            bytes[cursor + 5],
            bytes[cursor + 6],
            bytes[cursor + 7],
        ]);
        cursor += 8;
        let pos_1 = usize::from_le_bytes([
            bytes[cursor],
            bytes[cursor + 1],
            bytes[cursor + 2],
            bytes[cursor + 3],
            bytes[cursor + 4],
            bytes[cursor + 5],
            bytes[cursor + 6],
            bytes[cursor + 7],
        ]);
        cursor += 8;
        let char = bytes[cursor] as char;
        cursor += 1;
        let line = if bytes[cursor] == 1
        {
            cursor += 1;
            let len = usize::from_le_bytes([
                bytes[cursor],
                bytes[cursor + 1],
                bytes[cursor + 2],
                bytes[cursor + 3],
                bytes[cursor + 4],
                bytes[cursor + 5],
                bytes[cursor + 6],
                bytes[cursor + 7],
            ]);
            cursor += 8;
            let mut vec = Vec::with_capacity(len);
            for _ in 0..len
            {
                vec.push(bytes[cursor] as char);
                cursor += 1;
            }
            Some(vec)
        }
        else
        {
            None
        };
        Point {
            add,
            split,
            pos: (pos_0, pos_1),
            char,
            line,
        }
    }
}