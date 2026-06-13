//playback::音频播放引擎，封装 `rodio`
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use rodio::{Decoder, OutputStream, Sink};

pub struct RodioPlayer {
    // 必须保留 _stream 和 _handle，否则音频流会因为对象销毁而停止
    _stream: OutputStream,
    _handle: rodio::OutputStreamHandle,
    sink: Sink,
}

impl RodioPlayer {
    pub fn new() -> Result<Self, String> {
        // 初始化声卡驱动
        let (_stream, handle) = OutputStream::try_default()
            .map_err(|e| format!("无法找到音频输出设备: {}", e))?;
        
        let sink = Sink::try_new(&handle)
            .map_err(|e| format!("无法创建音频输出槽: {}", e))?;

        Ok(Self { _stream, _handle: handle, sink })
    }

    pub fn play_file(&mut self, path: &Path) -> Result<(), String> {
        // 1. 打开文件
        let file = File::open(path).map_err(|e| format!("文件打不开: {}", e))?;
        // 2. 解码音频文件
        let source = Decoder::new(BufReader::new(file))
            .map_err(|e| format!("音频格式不支持: {}", e))?;
        
        // 3. 停止当前播放并加入新歌
        self.stop(); 
        self.sink.append(source);
        self.sink.play();
        Ok(())
    }

    pub fn pause(&self) { self.sink.pause(); }
    pub fn resume(&self) { self.sink.play(); }
    pub fn stop(&mut self) { self.sink.stop(); }
    pub fn is_empty(&self) -> bool { self.sink.empty() }
}