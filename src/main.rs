use anyhow::Result;
use conmon::ConmonBuilder;

fn main() -> Result<()> {
    ConmonBuilder::default().build()?.run()
}
