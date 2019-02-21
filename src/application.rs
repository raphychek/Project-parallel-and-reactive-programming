#![recursion_limit = "256"]

extern crate crossbeam;

pub mod api;
pub mod common;
pub mod parallel;

#[cfg(test)]
mod tests {
    use super::api::prelude::*;
    use super::common::prelude::*;

    #[test]
    fn ssu() {
        use parallel::single_use::*;

        let mut x = None;
        let mut y = None;
        let mut z = None;

        {
            let x_ref = &mut x;
            let y_ref = &mut y;
            let z_ref = &mut z;

            let mut runtime = Toexec::new();

            let root = runtime.build_scope(|b| {
                let (setx_sender, setx_receiver) = b.port(None).split();
                let setx_activator = 
                    b.node(TaskNode {
                        inputs: (setx_receiver.as_data_input(),),
                        outputs: (),
                        task: StrictTask::new(move |x| *x_ref = x),
                    }).add_activator();
                let setx_input = setx_sender.with_activator(setx_activator);

                let (sety_sender, sety_receiver) = b.port(None).split();
                let sety_activator = 
                    b.node(TaskNode {
                        inputs: (sety_receiver.as_data_input(),),
                        outputs: (),
                        task: StrictTask::new(move |y| *y_ref = y),
                    })
                    .add_activator();
                let sety_input = sety_sender.with_activator(sety_activator);

                let (setz_sender, setz_receiver) = b.port(None).split();
                let setz_activator = 
                    b.node(TaskNode {
                        inputs: (setz_receiver.as_data_input(),),
                        outputs: (),
                        task: StrictTask::new(move |z| *z_ref = z),
                    })
                    .add_activator();
                let setz_input = setz_sender.with_activator(setz_activator);

                let (loop10_sender, loop10_receiver) = b.port(None).split();

                struct Loop10<O> {
                    data: i32,
                    output: O,
                };

                // In this implementation we take advantage of the fact that when scheduling
                // dynamic nodes, we can simply pass the data in the newly created task and don't
                // technically need to use any edges at all when there is no parallelism.
                impl<'r, O: OutputEdgeOnce<RuntimeLoc<'r>, Item = Option<i32>> + Send + Sync + 'r>
                    TaskOnce<(), (), RuntimeLoc<'r>> for Loop10<O>
                {
                    fn run_once(self, scheduler: &mut RuntimeLoc<'r>, _inputs: (), _outputs: ()) {
                        if self.data < 10 {
                            scheduler.schedule(Box::new(TaskNode {
                                inputs: (),
                                outputs: (),
                                task: Loop10 {
                                    data: self.data + 1,
                                    output: self.output,
                                },
                            }))
                        } else {
                            self.output.send_activate_once(scheduler, Some(self.data));
                        }
                    }
                }

                struct Loop10Init<O> {
                    output: O,
                };

                impl<
                        'r,
                        I: InputEdgeOnce<RuntimeLoc<'r>, Item = Option<i32>> + Send + Sync,
                        O: OutputEdgeOnce<RuntimeLoc<'r>, Item = Option<i32>> + Send + Sync + 'r,
                    > TaskOnce<(I,), (), RuntimeLoc<'r>> for Loop10Init<O>
                {
                    fn run_once(self, scheduler: &mut RuntimeLoc<'r>, inputs: (I,), _outputs: ()) {
                        let data = inputs.0.recv_activate_once(scheduler).unwrap();
                        scheduler.schedule(Box::new(TaskNode {
                            inputs: (),
                            outputs: (),
                            task: Loop10 {
                                data,
                                output: self.output,
                            },
                        }));
                    }
                }

                let loop10_activator = b
                    .node(TaskNode {
                        inputs: (loop10_receiver.as_data_input(),),
                        outputs: (),
                        task: Loop10Init { output: setz_input },
                    })
                    .add_activator();
                let loop10_input = loop10_sender.with_activator(loop10_activator);

                let (sender, receiver) = b.port(None).split();
                let mut identity = TaskNode {
                    inputs: (receiver.as_data_input(),),
                    // We use a `CloneOutput` edge which will automatically clone its output into each
                    // of the target nodes.
                    outputs: (CloneOutput::new(),),
                    task: StrictTask::new(|x| (x,)),
                };
                identity.outputs.0.connect(setx_input);
                identity.outputs.0.connect(sety_input);
                identity.outputs.0.connect(loop10_input);

                sender.with_activator(b.node(identity).add_activator())
            });
            root.send_activate_once(&mut runtime, Some(1));

            runtime.execute(5);
        }
        assert_eq!(x, Some(1));
        assert_eq!(y, Some(1));
        assert_eq!(z, Some(10));
    }
}