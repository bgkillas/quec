mod history;
mod misc;
use crate::{
    history::{History, Point},
    misc::{
        clear, clear_line, fix_history, fix_top, get_dimensions, get_word, help, open_file,
        print_line_number, read_single_char, save_file, Files,
    },
};
use console::Term;
use std::{
    cmp::min,
    env::{args, var},
    fs::create_dir,
    io::{stdout, Write},
};
fn main()
{
    let term = Term::stdout();
    let mut args = args().collect::<Vec<String>>();
    args.remove(0);
    let mut debug = false;
    'outer: loop
    {
        if args.is_empty()
        {
            break 'outer;
        }
        match args[0].as_str()
        {
            "--help" | "-h" =>
            {
                help();
                return;
            }
            "--version" | "-v" =>
            {
                println!("{} v{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
                return;
            }
            "--debug" => debug = true,
            _ => break,
        }
        args.remove(0);
    }
    let mut stdout = stdout();
    print!("\x1B[?1049h\x1B[H\x1B[J");
    stdout.flush().unwrap();
    #[cfg(unix)]
    let history_dir = var("HOME").unwrap() + "/.quec/";
    #[cfg(not(unix))]
    let history_dir = &format!(
        "C:\\Users\\{}\\AppData\\Roaming\\quec\\",
        var("USERNAME").unwrap()
    );
    let _ = create_dir(history_dir.clone());
    let (mut height, mut width) = get_dimensions();
    let mut files: Vec<Files> = Vec::new();
    for arg in args
    {
        files.push(open_file(arg, history_dir.clone()));
    }
    if files.is_empty()
    {
        files.push(Files {
            lines: vec![vec![]],
            history: History {
                pos: 0,
                list: vec![],
            },
            save_file_path: "".to_string(),
            history_file: "".to_string(),
            placement: 0,
            line: 0,
            start: 0,
            top: 0,
            cursor: 0,
        });
    }
    let mut n = 0;
    'outer: loop
    {
        let mut top = files[n].top;
        let mut start = files[n].start;
        clear(&files[n].lines, top, height, start, width);
        let mut line = files[n].line;
        let mut placement = files[n].placement;
        print_line_number(height, line, placement, top, start, String::new());
        stdout.flush().unwrap();
        let mut history = files[n].history.clone();
        let save_file_path = files[n].save_file_path.clone();
        let mut cursor = files[n].cursor;
        let mut c;
        let mut err = String::new();
        let mut edit = false;
        let mut search = false;
        let mut ln: (usize, usize) = (0, 0);
        let mut orig: (usize, usize) = (0, 0);
        let mut word: Vec<char> = Vec::new();
        let mut clip = Vec::new();
        let mut time = None;
        loop
        {
            if (height, width) != get_dimensions()
            {
                (height, width) = get_dimensions();
                top = fix_top(top, line, height);
                start = fix_top(start, placement, width);
                clear(&files[n].lines, top, height, start, width);
            }
            if history.list.len() >= 1000
            {
                history.list.drain(1000..);
            }
            if history.pos > history.list.len()
            {
                history.list.clear();
            }
            c = read_single_char(&term);
            if debug
            {
                time = Some(std::time::Instant::now());
            }
            match c
            {
                '\n' if !search =>
                {
                    //enter
                    if edit
                    {
                        line += 1;
                        if line == files[n].lines.len() && placement == 0
                        {
                            files[n].lines.push(Vec::new());
                            placement = 0;
                            cursor = placement;
                            if start != 0
                            {
                                start = 0;
                                if line == height + top
                                {
                                    top += 1;
                                }
                                clear(&files[n].lines, top, height, start, width);
                            }
                            else if line == height + top
                            {
                                top += 1;
                                clear(&files[n].lines, top, height, start, width);
                            }
                        }
                        else
                        {
                            let mut ln = files[n].lines[line - 1][placement..].to_vec();
                            files[n].lines.insert(line, ln);
                            ln = files[n].lines[line - 1][..placement].to_vec();
                            files[n].lines.insert(line, ln);
                            files[n].lines.remove(line - 1);
                            placement = 0;
                            cursor = placement;
                            start = 0;
                            if line == height + top
                            {
                                top += 1;
                            }
                            clear(&files[n].lines, top, height, start, width);
                        }
                        fix_history(&mut history);
                        history.list.insert(
                            0,
                            Point {
                                add: true,
                                split: true,
                                pos: (line, placement),
                                char: '\n',
                                line: None,
                            },
                        );
                    }
                }
                '\x08' =>
                {
                    //backspace
                    if edit
                    {
                        if placement == 0
                        {
                            if line == 0
                            {
                                continue;
                            }
                            line -= 1;
                            placement = files[n].lines[line].len();
                            let t = files[n].lines.remove(line + 1);
                            files[n].lines[line].extend(t);
                            start = fix_top(start, placement, width);
                            clear(&files[n].lines, top, height, start, width);
                            fix_history(&mut history);
                            history.list.insert(
                                0,
                                Point {
                                    add: false,
                                    split: true,
                                    pos: (line, placement),
                                    char: '\0',
                                    line: None,
                                },
                            )
                        }
                        else
                        {
                            placement -= 1;
                            fix_history(&mut history);
                            let ln = files[n].lines[line].remove(placement);
                            history.list.insert(
                                0,
                                Point {
                                    add: false,
                                    split: false,
                                    pos: (line, placement),
                                    char: ln,
                                    line: None,
                                },
                            );
                            if placement == files[n].lines[line].len()
                            {
                                if placement + 1 == start
                                {
                                    start -= 1;
                                    clear(&files[n].lines, top, height, start, width);
                                }
                                else
                                {
                                    print!("\x08\x1B[K");
                                }
                            }
                            else if placement + 1 == start
                            {
                                start -= 1;
                                clear(&files[n].lines, top, height, start, width);
                            }
                            else
                            {
                                print!("\x08");
                                clear_line(&files[n].lines, line, placement, width, start)
                            }
                        }
                        cursor = placement;
                    }
                    else if search && !word.is_empty()
                    {
                        word.pop();
                    }
                }
                '\x01' =>
                {
                    //home
                    placement = 0;
                    line = 0;
                    if files[n].lines.len() > height
                    {
                        top = 0;
                        start = fix_top(start, placement, width);
                        clear(&files[n].lines, top, height, start, width);
                    }
                    else
                    {
                        let s = start;
                        start = fix_top(start, placement, width);
                        if s != 0
                        {
                            clear(&files[n].lines, top, height, start, width);
                        }
                    }
                    cursor = placement;
                    if search
                    {
                        ln = (line, placement);
                    }
                }
                '\x02' =>
                {
                    //end
                    line = files[n].lines.len() - 1;
                    placement = files[n].lines[line].len();
                    if files[n].lines.len() > height
                    {
                        top = files[n].lines.len() - height;
                        start = fix_top(start, placement, width);
                        clear(&files[n].lines, top, height, start, width);
                    }
                    else
                    {
                        let s = start;
                        start = fix_top(start, placement, width);
                        if s != start
                        {
                            clear(&files[n].lines, top, height, start, width);
                        }
                    }
                    cursor = placement;
                    if search
                    {
                        ln = (line, placement);
                    }
                }
                '\x03' =>
                {
                    //page up
                    placement = 0;
                    if line < height
                    {
                        top = 0;
                        line = 0;
                        let s = start;
                        start = fix_top(start, placement, width);
                        if s != 0
                        {
                            clear(&files[n].lines, top, height, start, width);
                        }
                    }
                    else
                    {
                        line -= height;
                        if height < top
                        {
                            top -= height;
                        }
                        else
                        {
                            top = 0;
                        }
                        start = fix_top(start, placement, width);
                        clear(&files[n].lines, top, height, start, width);
                    }
                    cursor = placement;
                    if search
                    {
                        ln = (line, placement);
                    }
                }
                '\x04' =>
                {
                    //page down
                    if line + height >= files[n].lines.len()
                    {
                        top = files[n].lines.len().saturating_sub(height);
                        line = files[n].lines.len() - 1;
                        placement = files[n].lines[line].len();
                        start = fix_top(start, placement, width);
                        clear(&files[n].lines, top, height, start, width);
                    }
                    else
                    {
                        placement = 0;
                        line += height;
                        top += height;
                        if top + height > files[n].lines.len()
                        {
                            top = files[n].lines.len() - height;
                        }
                        start = fix_top(start, placement, width);
                        clear(&files[n].lines, top, height, start, width);
                    }
                    cursor = placement;
                    if search
                    {
                        ln = (line, placement);
                    }
                }
                '\x1B' | 'h' if c != 'h' || !edit && !search =>
                {
                    //left
                    if placement == 0
                    {
                        if line == 0
                        {
                            continue;
                        }
                        line -= 1;
                        placement = files[n].lines[line].len();
                        let s = start;
                        start = fix_top(start, placement, width);
                        if line + 1 == top
                        {
                            top -= 1;
                            if placement + 1 == start
                            {
                                start -= 1;
                            }
                            clear(&files[n].lines, top, height, start, width);
                        }
                        else if start != s
                        {
                            if placement + 1 == start
                            {
                                start -= 1;
                            }
                            clear(&files[n].lines, top, height, start, width);
                        }
                        else if placement + 1 == start
                        {
                            start -= 1;
                            clear(&files[n].lines, top, height, start, width);
                        }
                    }
                    else
                    {
                        placement -= 1;
                        if placement + 1 == start
                        {
                            start -= 1;
                            clear(&files[n].lines, top, height, start, width);
                        }
                    }
                    cursor = placement;
                    if search
                    {
                        ln = (line, placement);
                    }
                }
                '\x1C' | 'l' if c != 'l' || !edit && !search =>
                {
                    //right
                    if placement == files[n].lines[line].len()
                    {
                        if line + 1 != files[n].lines.len()
                        {
                            placement = 0;
                            line += 1;
                            let s = start;
                            start = fix_top(start, placement, width);
                            if line == height + top
                            {
                                top += 1;
                                if placement == width + start
                                {
                                    start += 1;
                                }
                                clear(&files[n].lines, top, height, start, width);
                            }
                            else if start != s
                            {
                                if placement == width + start
                                {
                                    start += 1;
                                }
                                clear(&files[n].lines, top, height, start, width);
                            }
                            else if placement == width + start
                            {
                                start += 1;
                                clear(&files[n].lines, top, height, start, width);
                            }
                        }
                    }
                    else
                    {
                        placement += 1;
                        if placement == width + start
                        {
                            start += 1;
                            clear(&files[n].lines, top, height, start, width);
                        }
                    }
                    cursor = placement;
                    if search
                    {
                        ln = (line, placement);
                    }
                }
                '\x1D' | 'k' if c != 'k' || !edit && !search =>
                {
                    //up
                    if line == 0
                    {
                        placement = 0;
                        cursor = 0;
                        if start != 0
                        {
                            start = 0;
                            if line + 1 == top
                            {
                                top -= 1;
                            }
                            clear(&files[n].lines, top, height, start, width);
                        }
                    }
                    else
                    {
                        line -= 1;
                        if cursor != 0
                        {
                            if files[n].lines[line].len() > cursor
                            {
                                placement = cursor;
                            }
                            else if placement < cursor || files[n].lines[line].len() < placement
                            {
                                if files[n].lines[line].is_empty()
                                {
                                    placement = 0;
                                }
                                else
                                {
                                    placement = files[n].lines[line].len();
                                }
                            }
                            let s = start;
                            start = fix_top(start, placement, width);
                            if s != start
                            {
                                if line + 1 == top
                                {
                                    top -= 1;
                                }
                                clear(&files[n].lines, top, height, start, width);
                            }
                            else if line + 1 == top
                            {
                                top -= 1;
                                clear(&files[n].lines, top, height, start, width);
                            }
                        }
                        else if line + 1 == top
                        {
                            top -= 1;
                            clear(&files[n].lines, top, height, start, width);
                        }
                    }
                    if search
                    {
                        ln = (line, placement);
                    }
                }
                '\x1E' | 'j' if c != 'j' || !edit && !search =>
                {
                    //down
                    if line + 1 == files[n].lines.len()
                    {
                        if !files[n].lines[line].is_empty()
                        {
                            placement = files[n].lines[line].len();
                            cursor = placement;
                            if line == height + top
                            {
                                top += 1;
                                clear(&files[n].lines, top, height, start, width);
                            }
                        }
                    }
                    else
                    {
                        line += 1;
                        if files[n].lines[line].is_empty()
                        {
                            placement = 0;
                        }
                        else if cursor != 0
                        {
                            if files[n].lines[line].len() > cursor
                            {
                                placement = cursor;
                            }
                            else if placement < cursor || files[n].lines[line].len() < placement
                            {
                                placement = files[n].lines[line].len();
                            }
                        }
                        let s = start;
                        start = fix_top(start, placement, width);
                        if s != start
                        {
                            if line == height + top
                            {
                                top += 1;
                            }
                            clear(&files[n].lines, top, height, start, width);
                        }
                        else if line == height + top
                        {
                            top += 1;
                            clear(&files[n].lines, top, height, start, width);
                        }
                    }
                    if search
                    {
                        ln = (line, placement);
                    }
                }
                '\x1A' =>
                {
                    //esc
                    edit = false;
                    search = false;
                    clear(&files[n].lines, top, height, start, width);
                }
                '`' if !edit && !search && n + 1 != files.len() =>
                {
                    //next file
                    files[n] = Files {
                        lines: files[n].lines.clone(),
                        history,
                        save_file_path,
                        history_file: files[n].history_file.clone(),
                        placement,
                        line,
                        start,
                        top,
                        cursor,
                    };
                    n += 1;
                    print!("\x1B[H\x1B[J");
                    stdout.flush().unwrap();
                    continue 'outer;
                }
                '~' if !edit && !search && n != 0 =>
                {
                    //last file
                    files[n] = Files {
                        lines: files[n].lines.clone(),
                        history,
                        save_file_path,
                        history_file: files[n].history_file.clone(),
                        placement,
                        line,
                        start,
                        top,
                        cursor,
                    };
                    n -= 1;
                    print!("\x1B[H\x1B[J");
                    stdout.flush().unwrap();
                    continue 'outer;
                }
                '0' if !edit && !search =>
                {
                    //start of line
                    placement = 0;
                    cursor = placement;
                    if start != 0
                    {
                        start = 0;
                        clear(&files[n].lines, top, height, start, width);
                    }
                    if search
                    {
                        ln = (line, placement);
                    }
                }
                '$' if !edit && !search =>
                {
                    //end of line
                    placement = files[n].lines[line].len();
                    cursor = placement;
                    if placement > start + width
                    {
                        start = placement - width + 1;
                        clear(&files[n].lines, top, height, start, width);
                    }
                    if search
                    {
                        ln = (line, placement);
                    }
                }
                'w' if !edit && !search =>
                {
                    //save
                    err = save_file(&mut files[n], history_dir.clone());
                    line = min(line, files[n].lines.len() - 1);
                    placement = min(placement, files[n].lines[line].len());
                    top = fix_top(top, line, height);
                    start = fix_top(start, placement, width);
                }
                'y' if !edit && !search =>
                {
                    //copy line
                    clip = files[n].lines[line].clone();
                }
                'd' if !edit && !search =>
                {
                    //cut line
                    placement = 0;
                    cursor = 0;
                    start = 0;
                    if line + 1 == files[n].lines.len()
                    {
                        clip = files[n].lines.pop().unwrap();
                        files[n].lines.push(Vec::new());
                        print!("\x1b[G\x1b[K");
                    }
                    else
                    {
                        clip = files[n].lines.remove(line);
                        clear(&files[n].lines, top, height, start, width);
                    }
                    fix_history(&mut history);
                    history.list.insert(
                        0,
                        Point {
                            add: false,
                            split: false,
                            pos: (line, placement),
                            char: '\0',
                            line: Some(clip.clone()),
                        },
                    )
                }
                'p' if !edit && !search =>
                {
                    //paste line
                    files[n].lines.insert(line, clip.clone());
                    placement = 0;
                    cursor = 0;
                    start = 0;
                    clear(&files[n].lines, top, height, start, width);
                    fix_history(&mut history);
                    history.list.insert(
                        0,
                        Point {
                            add: true,
                            split: false,
                            pos: (line, placement),
                            char: '\0',
                            line: Some(clip.clone()),
                        },
                    )
                }
                '/' if !edit && !search =>
                {
                    //enable search
                    search = true;
                    orig = (line, placement);
                    word = Vec::new()
                }
                'q' if !edit && !search =>
                {
                    //quit
                    print!("\x1B[G\x1B[{}B\x1B[?1049l", height);
                    stdout.flush().unwrap();
                    return;
                }
                'i' if !edit && !search =>
                {
                    //enable edit mode
                    edit = true;
                }
                'u' if !edit && !search && history.list.len() != history.pos =>
                {
                    //undo
                    match (
                        history.list[history.pos].add,
                        history.list[history.pos].split,
                        &history.list[history.pos].line,
                    )
                    {
                        (false, false, None) =>
                        {
                            line = history.list[history.pos].pos.0;
                            placement = history.list[history.pos].pos.1;
                            if line == files[n].lines.len()
                            {
                                files[n].lines.push(Vec::new());
                            }
                            files[n].lines[line].insert(placement, history.list[history.pos].char);
                            placement += 1;
                        }
                        (true, false, None) =>
                        {
                            line = history.list[history.pos].pos.0;
                            placement = history.list[history.pos].pos.1 - 1;
                            files[n].lines[line].remove(placement);
                        }
                        (false, true, None) =>
                        {
                            line = history.list[history.pos].pos.0 + 1;
                            placement = 0;
                            let l = files[n].lines[line - 1]
                                .drain(history.list[history.pos].pos.1..)
                                .collect();
                            files[n].lines.insert(line, l);
                        }
                        (true, true, None) =>
                        {
                            line = history.list[history.pos].pos.0 - 1;
                            placement = files[n].lines[line].len();
                            let l = files[n].lines.remove(line + 1);
                            files[n].lines[line].extend(l);
                        }
                        (false, false, Some(l)) =>
                        {
                            line = history.list[history.pos].pos.0;
                            placement = 0;
                            if line == files[n].lines.len()
                            {
                                files[n].lines.push(Vec::new());
                            }
                            files[n].lines.insert(line, l.clone());
                        }
                        (true, false, Some(_)) =>
                        {
                            line = history.list[history.pos].pos.0;
                            placement = 0;
                            files[n].lines.remove(line);
                        }
                        _ => unimplemented!(),
                    }
                    cursor = placement;
                    top = fix_top(top, line, height);
                    start = fix_top(start, placement, width);
                    clear(&files[n].lines, top, height, start, width);
                    history.pos += 1;
                }
                'U' if !edit && !search && history.pos > 0 =>
                {
                    //redo
                    history.pos -= 1;
                    match (
                        history.list[history.pos].add,
                        history.list[history.pos].split,
                        &history.list[history.pos].line,
                    )
                    {
                        (false, false, None) =>
                        {
                            line = history.list[history.pos].pos.0;
                            placement = history.list[history.pos].pos.1;
                            files[n].lines[line].remove(placement);
                        }
                        (true, false, None) =>
                        {
                            line = history.list[history.pos].pos.0;
                            placement = history.list[history.pos].pos.1 - 1;
                            files[n].lines[line].insert(placement, history.list[history.pos].char);
                            placement += 1;
                        }
                        (false, true, None) =>
                        {
                            line = history.list[history.pos].pos.0;
                            placement = files[n].lines[line].len();
                            let l = files[n].lines.remove(line + 1);
                            files[n].lines[line].extend(l);
                        }
                        (true, true, None) =>
                        {
                            line = history.list[history.pos].pos.0;
                            placement = 0;
                            if line == files[n].lines.len()
                            {
                                files[n].lines.push(Vec::new())
                            }
                            let l = files[n].lines[line]
                                .drain(history.list[history.pos].pos.1..)
                                .collect();
                            files[n].lines.insert(line, l);
                        }
                        (false, false, Some(_)) =>
                        {
                            line = history.list[history.pos].pos.0;
                            placement = 0;
                            files[n].lines.remove(line);
                        }
                        (true, false, Some(l)) =>
                        {
                            line = history.list[history.pos].pos.0;
                            placement = 0;
                            if line == files[n].lines.len()
                            {
                                files[n].lines.push(Vec::new());
                            }
                            files[n].lines.insert(line, l.clone());
                        }
                        _ => unimplemented!(),
                    }
                    cursor = placement;
                    top = fix_top(top, line, height);
                    start = fix_top(start, placement, width);
                    clear(&files[n].lines, top, height, start, width);
                }
                's' if !edit && !search =>
                {
                    match get_word(&term, &mut stdout, height)
                    {
                        Ok(file_path) =>
                        {
                            files[n].save_file_path = file_path;
                            err = save_file(&mut files[n], history_dir.clone());
                            line = min(line, files[n].lines.len() - 1);
                            placement = min(placement, files[n].lines[line].len());
                            top = fix_top(top, line, height);
                            start = fix_top(start, placement, width);
                        }
                        Err(()) =>
                        {
                            clear(&files[n].lines, top, height, start, width);
                        }
                    };
                }
                'o' if !edit && !search =>
                {
                    match get_word(&term, &mut stdout, height)
                    {
                        Ok(file_path) =>
                        {
                            let j = n;
                            for (index, file) in files.iter().enumerate()
                            {
                                if file.save_file_path == file_path
                                {
                                    n = index;
                                }
                            }
                            if n == j
                            {
                                files.push(open_file(file_path, history_dir.clone()));
                                n = files.len() - 1;
                            }
                            continue 'outer;
                        }
                        Err(()) =>
                        {
                            clear(&files[n].lines, top, height, start, width);
                        }
                    };
                }
                '\0' =>
                {}
                _ if !c.is_ascii()
                    || c.is_ascii_graphic()
                    || c == ' '
                    || c == '\t'
                    || c == '\n' =>
                {
                    if edit
                    {
                        files[n].lines[line].insert(placement, c);
                        if placement + 1 == width + start
                        {
                            placement += 1;
                            cursor = placement;
                            start += 1;
                            clear(&files[n].lines, top, height, start, width);
                        }
                        else
                        {
                            clear_line(&files[n].lines, line, placement, width, start);
                            placement += 1;
                            cursor = placement;
                        }
                        fix_history(&mut history);
                        history.list.insert(
                            0,
                            Point {
                                add: true,
                                split: false,
                                pos: (line, placement),
                                char: c,
                                line: None,
                            },
                        )
                    }
                    else if search
                    {
                        if c != '\n'
                        {
                            ln = orig;
                            word.push(c);
                        }
                        'inner: for (l, i) in files[n].lines.iter().enumerate()
                        {
                            if l >= ln.0 && word.len() <= i.len()
                            {
                                for j in
                                    if l == ln.0 { ln.1 + 1 } else { 0 }..=(i.len() - word.len())
                                {
                                    if i[j..j + word.len()] == word
                                    {
                                        ln = (l, j);
                                        (line, placement) = ln;
                                        top = fix_top(top, line, height);
                                        start = fix_top(start, placement, width);
                                        cursor = placement;
                                        clear(&files[n].lines, top, height, start, width);
                                        break 'inner;
                                    }
                                }
                                ln = orig;
                            }
                        }
                    }
                }
                _ =>
                {}
            }
            print_line_number(
                height,
                line,
                placement,
                top,
                start,
                if search
                {
                    word.iter().collect()
                }
                else if debug
                {
                    time.unwrap().elapsed().as_nanos().to_string()
                }
                else
                {
                    err.clone()
                },
            );
            stdout.flush().unwrap();
        }
    }
}