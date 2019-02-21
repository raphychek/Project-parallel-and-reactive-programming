// This sets the recursion limit when defining macros.  This is needed because the macro defining
// node traits for up to 10 inputs and 10 outputs calls itself recursively a large number of times.
#![recursion_limit = "256"]

extern crate crossbeam;

pub mod api;
pub mod common;
pub mod parallel;

#[cfg(test)]
mod tests {
    // Those tests build the following graph, where:
    //
    //  - Dup3 duplicate its input to multiple nodes
    //  - Set_X, Set_Y and Set_Z sets the x, y, and z variables respectively
    //  - Loop10 either activates its output iff the input is greater or equal to 10 and
    //    re-schedules itself with its input incremented by 1 otherwise.
    //
    //        1              ____
    //        |             /    \
    //        v            /      ^
    //   +----------+     /  +--------+   +-------+
    //   |   Dup3   |------->| Loop10 |-->| Set_Z |
    //   +----------+        +--------+   +-------+
    //        |      \
    //        |       \
    //        v        v
    //   +-----------+ +-------+
    //   |   Set_X   | | Set_Y |
    //   +-----------+ +-------+
    //

    use super::api::prelude::*;
    use super::common::prelude::*;
///*
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
                let setx_activator = //ScopedNodeBuilder::add_activator(&mut b
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

//*/

#[test]
// Demi-additionneur fait a la fin du projet, inspiré du TD4 compatible avec l'implémentation parallèle

fn demi_additionneur() {
    use parallel::single_use::*;

    let mut x: Option<bool> = None;
    let mut y: Option<bool> = None;

    {
        let x_ref = &mut x;
        let y_ref = &mut y;

        // début comme exemple précédent
        let mut runtime = Toexec::new();

        let root = runtime.build_scope(|b| {

            // x, y sont les booléens d'entrée de l'additionneur 

            let (setx_sender, setx_receiver) = b.port(None).split();

            let setx_activator = b.node(TaskNode {
                    inputs: (setx_receiver.as_data_input(),),
                    outputs: (),
                    task: StrictTask::new(move |x| *x_ref = x),
                }).add_activator();

            let setx_input = setx_sender.with_activator(setx_activator);


            let (sety_sender, sety_receiver) = b.port(None).split();

            let sety_activator = b.node(TaskNode {
                    inputs: (sety_receiver.as_data_input(),),
                    outputs: (),
                    task: StrictTask::new(move |y| *y_ref = y),
                }).add_activator();

            let sety_input = sety_sender.with_activator(sety_activator);

            // on crée le XOR
            let (xor_s1, xor_r1) = b.port(None).split();
            let (xor_s2, xor_r2) = b.port(None).split();

            let mut xor_node = b.node(TaskNode {
                inputs: (xor_r1.as_data_input(), xor_r2.as_data_input()),
                outputs: (setx_input,),
                task: StrictTask::new(|x :Option<bool>, y :Option<bool>| (Some(x.unwrap() ^ y.unwrap()),)),
            });

            let xor_activator1 = xor_node.add_activator();
            let xor_activator2 = xor_node.add_activator();
            let xor_input1 = xor_s1.with_activator(xor_activator1);
            let xor_input2 = xor_s2.with_activator(xor_activator2);

            // on crée le AND
            let (and_sender1, and_receiver1) = b.port(None).split();
            let (and_sender2, and_receiver2) = b.port(None).split();

            let mut and_node = b.node(TaskNode {
                inputs: (and_receiver1.as_data_input(), and_receiver2.as_data_input()),
                outputs: (sety_input,),
                task: StrictTask::new(|x :Option<bool>, y :Option<bool>| (Some(x.unwrap() & y.unwrap()),)),
            });
            let and_activator1 = and_node.add_activator();
            let and_activator2 = and_node.add_activator();

            let and_input1 = and_sender1.with_activator(and_activator1);
            let and_input2 = and_sender2.with_activator(and_activator2);

            // On envoie les booléens dans les entrées
            let (x_sender, x_receiver) = b.port(None).split();
            let mut x0 = TaskNode {
                inputs: (x_receiver.as_data_input(),),
                outputs: (CloneOutput::new(),),
                task: StrictTask::new(|x| (x,)),
            };
            x0.outputs.0.connect(xor_input1);
            x0.outputs.0.connect(and_input1);
            let x_input = x_sender.with_activator(b.node(x0).add_activator());

            let (y_sender, y_receiver) = b.port(None).split();
            let mut y0 = TaskNode {
                inputs: (y_receiver.as_data_input(),),
                outputs: (CloneOutput::new(),),
                task: StrictTask::new(|y| (y,)),
            };
            y0.outputs.0.connect(xor_input2);
            y0.outputs.0.connect(and_input2);
            let y_input = y_sender.with_activator(b.node(y0).add_activator());

            (x_input, y_input)
        });

        root.0.send_activate_once(&mut runtime, Some(true));
        root.1.send_activate_once(&mut runtime, Some(false));

        runtime.execute(2);
    }

    assert_eq!(x, Some(true));
    assert_eq!(y, Some(false));
}


    #[test]
    fn smu_dynamic() {
        use parallel::multiple_uses::*;

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
                let setx_activator = b
                    .node(TaskNode {
                        inputs: (setx_receiver.as_data_input(),),
                        outputs: (),
                        task: StrictTask::new(move |x| *x_ref = x),
                    })
                    .add_activator();
                let setx_input = setx_sender.with_activator(setx_activator);

                let (sety_sender, sety_receiver) = b.port(None).split();
                let sety_activator = b
                    .node(TaskNode {
                        inputs: (sety_receiver.as_data_input(),),
                        outputs: (),
                        task: StrictTask::new(move |y| *y_ref = y),
                    })
                    .add_activator();
                let sety_input = sety_sender.with_activator(sety_activator);

                let (setz_sender, setz_receiver) = b.port(None).split();
                let setz_activator = b
                    .node(TaskNode {
                        inputs: (setz_receiver.as_data_input(),),
                        outputs: (),
                        task: StrictTask::new(move |z| *z_ref = z),
                    })
                    .add_activator();
                let setz_input = setz_sender.with_activator(setz_activator);

                let (loop10_sender, loop10_receiver) = b.port(None).split();

                struct Loop10;

                // In this implementation, we pass the data as well as the final output (i.e. the
                // node to execute after the loop has ended) through dynamically created nodes.
                // Note that due to the "reusable" semantics of the runtime, we are executing
                // inside a `TaskMut`: if we wanted to store the data or the final output on the
                // Loop10 structure (as we do in the single-use implementation) we would need a way
                // to leave the `Loop10` in a valid state afterwards, for instance by using
                // options.  In this case, it doesn't matter, since the nodes we create will
                // actually be executed only once.

                impl<   'r,
                        I: InputEdgeOnce<RuntimeLoc<'r>, Item = Option<i32>> + Send + Sync,
                        L: InputEdgeOnce<RuntimeLoc<'r>, Item = Option<O>> + Send + Sync,
                        O: OutputEdgeOnce<RuntimeLoc<'r>, Item = Option<i32>> + Send + Sync + 'r,
                    > TaskMut<(I, L), (), RuntimeLoc<'r>> for Loop10
                {
                    fn run_mut(
                        &mut self,
                        scheduler: &mut RuntimeLoc<'r>,
                        inputs: (I, L),
                        _outputs: (),
                    ) {
                        let data = inputs.0.recv_activate_once(scheduler).unwrap();
                        let output = inputs.1.recv_activate_once(scheduler).unwrap();
                        if data < 10 {
                            let next_activator = scheduler.build_scope(|b| {
                                let (sender, receiver) = b.port(None).split();
                                sender.send_once(Some(data + 1));

                                let (output_sender, output_receiver) = b.port(None).split();
                                output_sender.send_once(Some(output));
                                b.node(TaskNode {
                                    inputs: (
                                        receiver.as_data_input(),
                                        output_receiver.as_data_input(),
                                    ),
                                    outputs: (),
                                    task: Loop10,
                                })
                                .add_activator()
                            });
                            next_activator.activate_once(scheduler);
                        } else {
                            output.send_activate_once(scheduler, Some(data));
                        }
                    }
                }

                let (loop10_output_send, loop10_output_recv) = b.port(None).split();
                loop10_output_send.send(Some(setz_input));

                let loop10_activator = b
                    .node(TaskNode {
                        inputs: (
                            loop10_receiver.as_data_input(),
                            loop10_output_recv.as_data_input(),
                        ),
                        outputs: (),
                        task: Loop10,
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
            root.send_activate(&mut runtime, Some(1));

            runtime.execute(5);
        }
        assert_eq!(x, Some(1));
        assert_eq!(y, Some(1));
        assert_eq!(z, Some(10));
    }


    #[test]
    fn smu_static() {
        use parallel::multiple_uses::*;

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
                let setx_activator = b
                    .node(TaskNode {
                        inputs: (setx_receiver.as_data_input(),),
                        outputs: (),
                        task: StrictTask::new(move |x| *x_ref = x),
                    })
                    .add_activator();
                let setx_input = setx_sender.with_activator(setx_activator);

                let (sety_sender, sety_receiver) = b.port(None).split();
                let sety_activator = b
                    .node(TaskNode {
                        inputs: (sety_receiver.as_data_input(),),
                        outputs: (),
                        task: StrictTask::new(move |y| *y_ref = y),
                    })
                    .add_activator();
                let sety_input = sety_sender.with_activator(sety_activator);

                let (setz_sender, setz_receiver) = b.port(None).split();
                let setz_activator = b
                    .node(TaskNode {
                        inputs: (setz_receiver.as_data_input(),),
                        outputs: (),
                        task: StrictTask::new(move |z| *z_ref = z),
                    })
                    .add_activator();
                let setz_input = setz_sender.with_activator(setz_activator);

                // This is a task which loops back its input and increments it until it reaches 10.  It can
                // only be used as a re-usable task.
                struct Inc10Task;

                impl<
                        I: InputEdgeOnce<S, Item = Option<i32>> + Sync + Send,
                        L: OutputEdgeOnce<S, Item = Option<i32>> + Sync + Send,
                        O: OutputEdgeOnce<S, Item = Option<i32>> + Sync + Send,
                        S,
                    > TaskMut<(I,), (L, O), S> for Inc10Task
                {
                    fn run_mut(&mut self, scheduler: &mut S, inputs: (I,), outputs: (L, O)) {
                        let data = inputs.0.recv_activate_once(scheduler).unwrap();
                        if data < 10 {
                            outputs.0.send_activate_once(scheduler, Some(data + 1));
                        } else {
                            outputs.1.send_activate_once(scheduler, Some(data));
                        }
                    }
                }

                let (loop_sender, loop_receiver) = b.port(None).split();
                let mut loop_node = b.node(TaskNode {
                    inputs: (loop_receiver.as_data_input(),),
                    outputs: (
                        loop_sender.clone().with_activator(Default::default()),
                        setz_input,
                    ),
                    task: Inc10Task,
                });
                // We build a Rc on top of a Rc because we use the refcount of the underlying
                // activator to set the pending count.
                let shared_activator = std::sync::Arc::new(loop_node.add_activator());
                loop_node.borrow_mut().outputs.0.activator = shared_activator.clone();

                // As an alternative, we implement the identity with a `dup3` node which
                // explicitely duplicates its input instead of using a dynamic `CloneOutput` edge.
                let (sender, receiver) = b.port(None).split();
                let loop_input = loop_sender.with_activator(shared_activator);
                let identity_activator = b
                    .node(TaskNode {
                        inputs: (receiver.as_data_input(),),
                        outputs: (setx_input, sety_input, loop_input),
                        task: StrictTask::new(|x: Option<i32>| (x.clone(), x.clone(), x)),
                    })
                    .add_activator();

                sender.with_activator(identity_activator)
            });
            root.send_activate(&mut runtime, Some(1));

            runtime.execute(5);
        }

        assert_eq!(x, Some(1));
        assert_eq!(y, Some(1));
        assert_eq!(z, Some(10));
  
  }
}