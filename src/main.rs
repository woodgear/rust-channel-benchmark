use crossbeam_channel::{self, unbounded};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

#[derive(Debug)]
struct Msg(Instant, u32, Vec<u8>);

fn benchmark<S, R>(
    sender_count: u32,
    items_count: u32,
    payload_size: u32,
    c: (S, R),
) -> Vec<Duration>
where
    S: SenderTrait<Msg> + Send + Clone + 'static,
    R: ReceiverTrait<Msg> + Send + 'static,
{
    let (s, r) = c;
    let payload: Vec<u8> = (0..payload_size).map(|_| 0).collect();
    let sender_thread_pool: Vec<JoinHandle<()>> = (0..sender_count)
        .map(|id| {
            let s = s.clone();
            let payload = payload.clone();
            thread::spawn(move || {
                for i in 0..items_count {
                    let msg = Msg(Instant::now(), id, payload.clone());
                    s.send(msg);
                }
            })
        })
        .collect();
    drop(s);
    let main = thread::spawn(move || {
        let mut res = vec![];
        while let Ok(m) = r.recv() {
            let dur = Instant::now() - m.0;
            println!("{:?} {}", dur, dur.as_nanos());
            res.push(dur);
        }
        res
    });
    main.join().unwrap()
}

trait SenderTrait<T> {
    fn send(&self, t: T) -> Result<(), String>;
}

trait ReceiverTrait<T> {
    fn recv(&self) -> Result<T, String>;
}

impl<T> SenderTrait<T> for Sender<T> {
    fn send(&self, t: T) -> Result<(), String> {
        self.send(t).map_err(|e| e.to_string())
    }
}

impl<T> ReceiverTrait<T> for Receiver<T> {
    fn recv(&self) -> Result<T, String> {
        self.recv().map_err(|e| e.to_string())
    }
}

impl<T> SenderTrait<T> for crossbeam_channel::Sender<T> {
    fn send(&self, t: T) -> Result<(), String> {
        self.send(t).map_err(|e| e.to_string())
    }
}

impl<T> ReceiverTrait<T> for crossbeam_channel::Receiver<T> {
    fn recv(&self) -> Result<T, String> {
        self.recv().map_err(|e| e.to_string())
    }
}

fn render_echarts(data: Vec<(Vec<Duration>, String)>, path: &str) {
    use serde_json::json;
    let series: serde_json::Value = data
        .iter()
        .map(|(line, name)| {
            let data: Vec<u128> = line.iter().map(|d| d.as_nanos()).collect();
            json!({
                "name":name,
                "data":data,
                "type":"line",
                "lable":{"show":true},
            })
        })
        .collect();

    let js = format!(
        r#"option = {{
            "xAxis": {{
                "type": "category",
                "data": []
            }},
            "legend": {{
            
            }},
            "yAxis": {{
                "type": "value",
                "axisLabel":{{
                    "formatter":function (s) {{
                            function convert(s,n) {{
                            return  s*1.0/Math.pow(10,n)
                          }}
                          cmds = [
                            ["s",(s)=>convert(s,9)],
                            ["ms",(s)=>convert(s,6)],
                            ["us",(s)=>convert(s,3)],
                            ["ns",(s)=>convert(s,0)],
                          ]
                          const res = cmds.map(([fmt,f])=>{{
                            const time_fmt = f(s);
                            return [fmt,time_fmt]
                          }})
                          .filter(([fmt,time])=>{{
                            return time>1;
                          }})[0];
                          return !!!res?'0s':`${{res[1]}}${{res[0]}}`;
                          }}
                }}
            }},
            "series": {}
        }}"#,
        series.to_string()
    );
    println!("{}", js.to_string());
    std::fs::write(path, js.to_string().as_bytes());
}

fn main() {
    render_echarts(
        vec![
            (
                benchmark(10, 10, 100, channel()),
                "std-channel-10-thread-10-itemspre-100-payloadsize".to_owned(),
            ),
            (
                benchmark(10, 10, 100, unbounded()),
                "crossbeam-channel-10-thread-10-itemspre-100-payloadsize".to_owned(),
            ),
            (
                benchmark(1, 100, 100, channel()),
                "std-channel-1-thread-100-itemspre-100-payloadsize".to_owned(),
            ),
            (
                benchmark(1, 100, 100, unbounded()),
                "crossbeam-channel-1-thread-100-itemspre-100-payloadsize".to_owned(),
            ),
        ],
        "echarts.js",
    );
}
