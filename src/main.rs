use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use rodio::Source;
use std::fs::File;
use std::io::{BufReader, Read};
use std::time::{SystemTime, UNIX_EPOCH};
use std::{io, time::Duration};

use tui::widgets::{Gauge, Wrap};
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Span, Text},
    widgets::{Block, Borders, Paragraph, Widget},
    Frame, Terminal,
};
use tui_textarea::{CursorMove, TextArea};

/*
定义结构体 存放用户输入的可视化数据
 */
#[derive(Debug)]
struct CoolView {
    total: i32,
    // 总数
    word_count: i32,
    // 已输入的字符数
    start_time: u64,
    // 开始时间
    mistake_count: i32,
    // 输入错误的次数
    precise_count: i32, // 输入正确的次数
}

fn main() -> Result<(), io::Error> {
    // getData();

    // 初始化终端
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // 渲染界面
    run_app(&mut terminal)?;
    // 恢复终端
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}

/*
发起http请求获取乱数假文
*/
fn _get_date() -> String {
    let resp = reqwest::blocking::get("http://www.atoolbox.net/Api/GetLoremIpsum.php?p=5")
        .unwrap()
        .text()
        .unwrap();
    resp.replace("<p>", "")
        .replace("</p>", "")
        .replace("\n", "")
        .clone()
}

// 逻辑主方法
fn run_app<B: Backend>(terminal: &mut Terminal<B>) -> io::Result<()> {
    // 读取文本内容
    // let string1 = _get_date();
    let mut file = std::fs::File::open("data.txt").expect("data.txt is not found");
    // file.write_all(string1.as_bytes()).unwrap();
    let mut string = String::new();
    // 展示的字符串
    file.read_to_string(&mut string).expect("Read Error");

    // 定义用户输入的内容
    let mut user_input = String::new();
    // 定义编辑框
    let mut textarea = TextArea::default();
    // 禁用当前行的下划线显示
    // textarea.set_line_number_style(Style::default().fg(Color::Green));

    // 标识用户有无错误输入
    let mut b = false;
    // 初始化结构体
    let mut cool = CoolView {
        total: string.len() as i32,
        word_count: 0,
        start_time: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        mistake_count: 0,
        precise_count: 0,
    };
    // 准备音频流 OutputStream流不可丢弃 虽然我们没有调用到
    let (_stream, stream_handle) = rodio::OutputStream::try_default().unwrap();

    // 定义播放输入正确声音的闭包
    let play_success = || {
        let file = File::open("char.mp3").unwrap();
        let reader = BufReader::new(file);
        let source = rodio::Decoder::new(reader).unwrap();
        stream_handle
            .play_raw(source.convert_samples())
            .expect("播放输入正确音效失败");
    };
    // 定义输入错误播放声音的闭包
    let play_fail = || {
        let source = rodio::Decoder::new(BufReader::new(File::open("fail.wav").unwrap())).unwrap();
        stream_handle
            .play_raw(source.convert_samples())
            .expect("播放输入错误音效失败");
    };
    // 定义输入Enter播放声音的闭包
    let play_enter = || {
        let source = rodio::Decoder::new(BufReader::new(File::open("enter.wav").unwrap())).unwrap();
        stream_handle
            .play_raw(source.convert_samples())
            .expect("播放回车键音效播放失败");
    };
    // 定义输入backspace播放声音的闭包
    let play_backspace = || {
        let source = rodio::Decoder::new(BufReader::new(File::open("backspace.wav").unwrap())).unwrap();
        stream_handle
            .play_raw(source.convert_samples())
            .expect("播放删除键音效播放失败");
    };

    loop {
        // 处理按键事件
        if event::poll(Duration::from_secs(1))? {
            if let Event::Key(key) = event::read()? {
                // 匹配按键
                match key.code {
                    // 按下字符键
                    KeyCode::Char(ch) => {
                        // 添加用户输入的内容
                        textarea.input(key);
                        user_input = user_input + &*String::from(ch);
                        // 处理用户正确与错误输入
                        let len = user_input.len();
                        let string_exist = &string[0..len];
                        if string_exist.eq(user_input.as_str()) {
                            b = true;
                            cool.precise_count = cool.precise_count + 1;
                            // 播放正确的音效
                            play_success();
                        } else {
                            b = false;
                            cool.mistake_count = cool.mistake_count + 1;
                            // 播放失败的音效
                            play_fail();
                        }
                        cool.word_count = cool.word_count + 1;
                    }
                    // 按下回车键 意味着开始输入下一行的字了
                    KeyCode::Enter => {
                        // 数据切片
                        let x = &string[user_input.len()..string.len()];
                        string = x.parse().unwrap();
                        // 清空textArea
                        textarea.delete_line_by_end();
                        textarea.delete_line_by_head();
                        // 清除已经输入过的内容
                        user_input = String::new();
                        // 播放回车键的音效
                        play_enter();
                    }
                    /*
                    原有的按键逻辑非正常 自定义一映射
                    */
                    // 按下删除键
                    KeyCode::Backspace => {
                        textarea.delete_char(); // 删除光标前的一个字符
                                                // 判断一下防止下标为负数越界
                        if user_input.len().ne(&0) {
                            let result = &user_input[0..user_input.len() - 1];
                            user_input = result.parse().unwrap();
                        }
                        // 播放删除键音效
                        play_backspace();
                    }
                    // <- 将光标向前移动一个字符
                    KeyCode::Left => {
                        textarea.move_cursor(CursorMove::Back);
                    }
                    // -> 将光标向后移动一个字符
                    KeyCode::Right => {
                        textarea.move_cursor(CursorMove::Forward);
                    }
                    // Esc键退出
                    KeyCode::Esc => {
                        break;
                    }
                    _ => {}
                }
            }
        }
        // 渲染页面
        terminal.draw(|f| ui(f, &mut string, &mut textarea, &mut user_input, &mut cool, b))?;
    }
    Ok(())
}

/**
渲染UI界面
 **/
fn ui<B: Backend>(
    f: &mut Frame<B>,
    string: &mut String,
    textarea: &mut TextArea,
    user_input: &mut String,
    cool: &mut CoolView,
    b: bool,
) {
    // 布局
    let chunks = Layout::default() // 首先获取默认构造
        .constraints(
            [
                Constraint::Percentage(30), // 正文显示
                Constraint::Percentage(20), // 用户输入的数据展示
                Constraint::Percentage(20), // 编辑框
                Constraint::Percentage(20), // 进度条
                Constraint::Percentage(10), // 左右护法
            ]
            .as_ref(),
        ) // 按照 10%划分  划分了8个区域 再加一个20%的文本编辑框
        .direction(Direction::Vertical) // 垂直分割
        .split(f.size()); // 分割整块 Terminal 区域

    // 展示正文
    let mut content = string.clone();
    let paragraph = Paragraph::new(Span::styled(
        content,
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD), // 设置字体样式
    ))
    // .block(Block::default().borders(Borders::ALL).border_style(Style::fg(Default::default(), Color::LightGreen)).title("TypingPractice OK"))//设置区块标题
    .wrap(Wrap { trim: true }) // 文本内容换行展示
    .alignment(tui::layout::Alignment::Left); // 对齐方式
    f.render_widget(paragraph, chunks[0]);

    // 用户输入展示
    content = user_input.clone();
    let user_paragraph;
    // 根据用户输入的正确性 来决定展示的组件样式
    if b {
        user_paragraph = Paragraph::new(Span::styled(
            content,
            Style::default()
                .fg(Color::LightGreen)
                .add_modifier(Modifier::SLOW_BLINK), // 设置字体样式
        ))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::fg(Default::default(), Color::LightGreen))
                .title("TypingPractice OK"),
        ) //设置区块标题
        .alignment(tui::layout::Alignment::Center);
    } else {
        user_paragraph = Paragraph::new(Span::styled(
            content,
            Style::default()
                .fg(Color::LightRed)
                .add_modifier(Modifier::SLOW_BLINK), // 设置字体样式
        ))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::fg(Default::default(), Color::LightRed))
                .title("TypingPractice ERR"),
        ) //设置区块标题
        .alignment(tui::layout::Alignment::Center);
    }
    f.render_widget(user_paragraph, chunks[1]);

    // 添加文本编辑区域
    let widget = textarea.widget();
    f.render_widget(widget, chunks[2]);

    // 添加视图区域
    // 完成进度
    let label = format!("{} / {}", cool.word_count, cool.total);
    let gauge = Gauge::default()
        .block(
            Block::default()
                .title("Complete schedule")
                .borders(Borders::ALL),
        )
        .gauge_style(Style::default().fg(Color::Magenta).bg(Color::LightCyan))
        .percent((f64::from(cool.word_count) / f64::from(cool.total) * 100.0) as u16) // 这个就是进度条的进度百分比 0..100
        .label(label); // 进度条中的内容
    f.render_widget(gauge, chunks[3]);

    // 错误和正确的展示区域
    // 拆分为两块
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage(38),
                Constraint::Percentage(28),
                Constraint::Percentage(38),
            ]
            .as_ref(),
        )
        .split(chunks[4]);

    // 正确的
    let left_paragraph = Paragraph::new(Span::styled(
        "PRECISE:".to_string() + &*cool.precise_count.to_string(),
        Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD), // 设置字体样式
    ))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::fg(Default::default(), Color::Green))
            .title("PRECISE NUM"),
    ) //设置区块标题
    .alignment(tui::layout::Alignment::Center); // 对齐方式
    f.render_widget(left_paragraph, chunks[0]);

    // 中间放个打字速度
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let mut sec = (now - cool.start_time) as i32;
    if sec == 0 {
        sec = 1;
    } // 防止除以0
    let i = cool.word_count / sec * 60;
    let content = format!("{} character in 1min", i);
    let center_paragraph = Paragraph::new(Span::styled(
        content,
        Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD), // 设置字体样式
    ))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::fg(Default::default(), Color::LightBlue))
            .title("SPEED"),
    ) //设置区块标题
    .alignment(tui::layout::Alignment::Center); // 对齐方式
    f.render_widget(center_paragraph, chunks[1]);

    // 错误的
    let right_paragraph = Paragraph::new(Span::styled(
        "MISTAKE:".to_string() + &*cool.mistake_count.to_string(),
        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD), // 设置字体样式
    ))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::fg(Default::default(), Color::Red))
            .title("MISTAKE NUM"),
    ) //设置区块标题
    .alignment(tui::layout::Alignment::Center); // 对齐方式
    f.render_widget(right_paragraph, chunks[2]);
}
