pub mod payments
{
    use std::collections::{HashMap, HashSet};
    use std::fs::File;
    use std::io::{BufWriter, BufReader};
    use regex::Regex;
    use serde::{Serialize, Deserialize};

    type PaymentResult = Result<(), String>;
    
    #[derive(Debug, Deserialize, Serialize)]
    struct Participant
    {
        pub name: String,
        pub tasks: HashSet<String>,
        pub paid_tasks: HashSet<String>,
        pub payments_made: Option<Vec<i32>>,
        #[serde(skip)]
        pub sum: Option<f32>,
    }

    impl Participant
    {
        pub fn new (name: &str) -> Self
        {
            Self
            {
                name: String::from (name),
                tasks: HashSet::new (),
                paid_tasks: HashSet::new (),
                payments_made: Some (Vec::new ()),
                sum: None,
            }
        }
    }
    
    #[derive(Debug, Deserialize, Serialize)]
    struct Task
    {
        pub name: String,
        pub owner: String,
        pub participants: HashSet<String>,
        pub cost: i32,
    }

    #[derive(Debug, Deserialize, Serialize)]
    pub struct Payment
    {
        participants: HashMap<String, Participant>,
        tasks: HashMap<String, Task>,
    }

    impl Payment
    {
        pub fn new () -> Self
        {
            Self {
                participants: HashMap::new (),
                tasks: HashMap::new ()
            }
        }

        pub fn command (&mut self, com: &str) -> PaymentResult
        {
            let parts = Regex::new (r"\s+")
                .unwrap ()
                .split (com)
                .collect::<Vec<&str>> ();
            let end = &parts[1..parts.len () - 1];
            match parts.get (0)
            {
                Some (&"add")     => return self.add (end),
                Some (&"part")    => return self.part (end),
                Some (&"payment") => return self.payment (end),
                Some (&"pay")     => return self.pay (end),
                Some (&"print")   => self.print (end),
                Some (&"save")    => return self.save (end),
                Some (&"load")    => return self.load (end),
                Some (&"rename")  => return self.rename (end),
                Some (&"remove")  => return self.remove (end),
                Some (&a)         => return Err (format! ("{} is not recognized as a command", a)),
                None              => return Err (String::from ("syntax error")),
            }
            Ok (())
        }

        fn load (&mut self, args: &[&str]) -> PaymentResult
        {
            let filename = match args.get (0)
            {
                Some (&f) => f,
                None => return Err (String::from ("Not enough arguments")),
            };
            let file = match File::open (filename)
            {
                Ok (f) => f,
                Err (_) => return Err (format! ("Unable to open file {}", filename)),
            };
            let payment: Payment = match serde_json::from_reader (BufReader::new (file))
            {
                Ok (pay) => pay,
                Err (e) => return Err (format! ("Error deserializing file:\n{}", e)),
            };
            self.participants = payment.participants;
            self.tasks = payment.tasks;
            Ok (())
        }


        fn save (&mut self, args: &[&str]) -> PaymentResult
        {
            self.calculate ();
            match args.get (0)
            {
                Some (&s) => return self.save_file (s),
                None => return self.save_string (),
            }
        }

        fn save_file (&self, filename: &str) -> PaymentResult
        {
            let file = match File::create (filename)
            {
                Ok (f) => f,
                Err (_) => return Err (format! ("Unable to open file {}", filename)),
            };
            let write = BufWriter::new (file);
            match serde_json::to_writer_pretty (write, &self)
            {
                Ok (_) => Ok (()),
                Err (e) => Err (format! ("Error serializing the object:\n{}", e)),
            }
        }
        
        fn save_string (&self) -> PaymentResult
        {
            let json = match serde_json::to_string_pretty (&self)
            {
                Ok (val) => val,
                Err (e) => return Err (format! ("Something went wrong serializing the object:\n{}", e)),
            };
            println! ("{}", json);
            Ok (())
        }

        fn rename (&mut self, args: &[&str]) -> PaymentResult
        {
            if args.len () != 2
            {
                return Err (String::from ("remove must be called with 2 arguments"));
            }
            if self.participants.contains_key (args[1]) || self.tasks.contains_key (args[1])
            {
                return Err (format! ("{} already exists", args[1]));
            }
            // see if we are renaming a participant
            if let Some (mut part) = self.participants.remove (args[0])
            {
                // renaming a participant
                for task_name in &part.tasks
                {
                    let task = self.tasks.get_mut (task_name).unwrap ();
                    task.participants.remove (&part.name);
                    task.participants.insert (String::from (args[1]));
                    if part.paid_tasks.contains (task_name)
                    {
                        task.owner = String::from (args[1]);
                    }
                }
                part.name = String::from (args[1]);
                self.participants.insert (String::from (args[1]), part);

                return Ok (())
            }
            // check if there is a task with this name
            if let Some (mut task) = self.tasks.remove (args[0])
            {
                for name in &task.participants
                {
                    let part = self.participants.get_mut (name).unwrap ();
                    part.tasks.remove (&task.name);
                    part.tasks.insert (String::from (args[1]));
                    if part.paid_tasks.remove (&task.name)
                    {
                        part.paid_tasks.insert (String::from (args[1]));
                    }
                }
                task.name = String::from (args[1]);
                self.tasks.insert (String::from (args[1]), task);

                return Ok (());
            }
            // nothing can be renamed, return error
            return Err (format! ("No task or participant found named {}", args[0]));
        }

        fn remove_from (&mut self, part_name: &str, task_name: &str) -> PaymentResult
        {
            if part_name == "all" || task_name == "all"
            {
                return Err (String::from ("Unable to remove all tasks"));
            }

            let Some (part) = self.participants.get_mut (part_name) else
            {
                return Err (format! ("No participant named {part_name} exists"));
            };
            if part.paid_tasks.contains (task_name)
            {
                return Err (format! ("{part_name} paid for {task_name}, remove {task_name} instead"));
            }
            if part.tasks.remove (task_name)
            {
                // this task exists since it's listed as a task for the participant
                let task = self.tasks.get_mut (task_name).unwrap ();
                task.participants.remove (part_name);
            }
            Ok (())
        }

        fn remove (&mut self, args: &[&str]) -> PaymentResult
        {
            if args.len () == 2
            {
                return self.remove_from (args[0], args[1]);
            }
            if args.len () != 1
            {
                return Err (String::from ("Wrong number of arguments"));
            }
            // check if the removal is a participant
            if let Some (part) = self.participants.remove (args[0])
            {
                // remove this participant from all of their tasks
                for task_name in &part.tasks
                {
                    if part.paid_tasks.contains (task_name)
                    {
                        continue;
                    }
                    let task = self.tasks.get_mut (task_name).unwrap ();
                    task.participants.remove (&part.name);
                }
                // remove all tasks this participant owns
                for task_name in &part.paid_tasks
                {
                    self.remove_task (task_name)?;
                }
                return Ok (());
            }
            match self.remove_task (args[0])
            {
                Ok (_) => Ok (()),
                Err (_) => Err (format! ("{} is not a task or participant", args[0])),
            }
        }

        fn remove_task (&mut self, task_name: &str) -> PaymentResult
        {
            if task_name == "all"
            {
                return Err (String::from ("Unable to remove all tasks"));
            }
            let Some (task) = self.tasks.remove (task_name) else
            {
                return Err (format! ("Task {} was not present to be removed", task_name));
            };
            for name in &task.participants
            {
                let Some (part) = self.participants.get_mut (name) else
                {
                    continue;
                };
                part.tasks.remove (task_name);
                part.paid_tasks.remove (task_name);
            }
            Ok (())
        }

        fn calculate (&mut self)
        {
            for part in self.participants.values_mut ()
            {
                let mut sum = 0f32;
                for task_name in &part.tasks
                {
                    // divide the cost of this task among its participants and
                    // add that amount to the amount this participant owes
                    let task = self.tasks.get (task_name).unwrap ();
                    sum += task.cost as f32 / task.participants.len () as f32;
                }
                for task_name in &part.paid_tasks
                {
                    // same as before but subtracting since this participant
                    // has already paid their share of this task
                    let task = self.tasks.get (task_name).unwrap ();
                    sum -= task.cost as f32;
                }
                if let Some (amounts) = &part.payments_made
                {
                    for &amount in amounts
                    {
                        sum -= amount as f32;
                    }
                }
                part.sum = Some (sum.round () / 100f32);
            }
        }

        fn print_participant (&self, part: &Participant)
        {
            println! ("{} owes {}", part.name, part.sum.unwrap ());
            if !part.tasks.is_empty ()
            {
                println! ("  participated in:");
            }
            for task_name in &part.tasks
            {
                let task = self.tasks.get (task_name).unwrap ();
                println! ("    {task_name}: {} / {} = {}"
                          , task.cost as f32 / 100f32
                          , task.participants.len ()
                          , (task.cost as f32
                             / task.participants.len () as f32).round ()
                          / 100f32);
            }
            if !part.paid_tasks.is_empty ()
            {
                println! ("  paid for:");
            }
            for task_name in &part.paid_tasks
            {
                println! ("    {task_name}: {}",
                          self.tasks.get (task_name)
                          .unwrap ()
                          .cost as f32 / 100f32);
            }
            let Some (amounts) = &part.payments_made else
            {
                return;
            };
            if !amounts.is_empty ()
            {
                println! ("  has paid:");
            }
            for &amount in amounts
            {
                println! ("    {}", amount as f32 / 100f32);
            }
        }

        fn print_task (&self, task: &Task)
        {
            println! ("{} paid {} for {}", task.owner, task.cost as f32 / 100f32, task.name);
            println! ("  participants: {}", task.participants.len ());
            for part in &task.participants
            {
                println! ("    {}", part);
            }
        }

        fn print (&mut self, args: &[&str])
        {
            self.calculate ();
            let mut normal = true;
            for &arg in args
            {
                if arg == "-a"
                {
                    for part in self.participants.values ()
                    {
                        self.print_participant (&part);
                    }
                    return;
                }
                if arg == "-t"
                {
                    for task in self.tasks.values ()
                    {
                        self.print_task (&task);
                    }
                    return;
                }
                let part = match self.participants.get (arg)
                {
                    Some (val) => val,
                    None => continue,
                };
                self.print_participant (&part);
                normal = false;
            }
            if normal
            {
                for part in self.participants.values ()
                {
                    println! ("{} owes {}", part.name, part.sum.unwrap ());
                }
                return;
            }
        }

        fn add (&mut self, args: &[&str]) -> PaymentResult
        {
            if args.is_empty ()
            {
                return Err (String::from ("Not enough arguments"));
            }
            for &arg in args
            {
                let name = match arg
                {
                    "" => return Err (String::from ("Not enough arguments")),
                    "-a" => return Err (String::from ("invalid name")),
                    "all" => continue,
                    n => n,
                };
                // if there is already a participant with this name, we don't want
                // to overwrite them
                if self.participants.contains_key (name)
                {
                    return Err (format! ("participant {name} was already added"));
                }
                if self.tasks.contains_key (name)
                {
                    return Err (format! ("A task named {name} exists"));
                }
                self.participants.insert (String::from (name), Participant::new (name));
            }
            Ok (())
        }

        fn pay (&mut self, args: &[&str]) -> PaymentResult
        {
            let name = match args.get (0)
            {
                Some (&"") => return Err (String::from ("Not enough arguments")),
                Some (&"all") => return Err (String::from ("Cannot use all here")),
                Some (&n) => n,
                None => return Err (String::from ("Not enough arguments")),
            };
            let task_name = match args.get (1)
            {
                Some (&"") => return Err (String::from ("Not enough arguments")),
                Some (&"all") => return Err (String::from ("Cannot use all here")),
                Some (&n) => n,
                None => return Err (String::from ("Not enough arguments")),
            };
            if self.participants.contains_key (task_name)
            {
                return Err (format! ("Cannot add {task_name}, a participant exists with that name"));
            }
            let price_string = match args.get (2)
            {
                Some (&"") => return Err (String::from ("Not enough arguments")),
                Some (&n) => n,
                None => return Err (String::from ("Not enough arguments")),
            };
            let price = match price_string.parse::<f32> ()
            {
                Ok (p) => p,
                Err (_) => return Err (format! (
                        "{} not a valid decimal number for the price"
                        , price_string)),
            };
            // if this participant doesn't yet exist, add them
            if !self.participants.contains_key (name)
            {
                self.participants.insert (String::from (name), Participant::new (name));
            }
            // see if the task is being edited or added
            let task = match self.tasks.get_mut (task_name)
            {
                Some (val) =>
                {
                    // this task already exists, check if the owner should be
                    // changed and change the cost
                    val.cost = (price * 100f32) as i32;
                    if val.owner != name
                    {
                        let owner = &val.owner;
                        let part = self.participants.get_mut (owner).unwrap ();
                        part.tasks.remove (&val.name);
                        part.paid_tasks.remove (&val.name);
                        val.participants.remove (owner);
                        val.owner = String::from (name);
                    }
                    val
                },
                None =>
                {
                    self.tasks.insert (String::from (task_name), Task
                                  {
                                      name: String::from (task_name),
                                      owner: String::from (name),
                                      participants: HashSet::new (),
                                      cost: (price * 100f32) as i32,
                                  });
                    self.tasks.get_mut (task_name).unwrap ()
                },
            };
            // add this task to the paid tasks of the participant
            let participant = self.participants.get_mut (name).unwrap ();
            participant.paid_tasks.insert (String::from (task_name));
            participant.tasks.insert (String::from (task_name));
            //self.tasks.get_mut (task_name)
            //    .unwrap ()
                task.participants.insert (String::from (name));
            Ok (())
        }

        fn part (&mut self, args: &[&str]) -> PaymentResult
        {
            if args.len () <= 1
            {
                return Err (String::from ("Not enough arguments"));
            }
            let task_name = match args.get (0)
            {
                Some (&"") => return Err (String::from ("Not enough arguments")),
                Some (&n) => n,
                None => return Err (String::from ("Not enough arguments")),
            };

            if task_name == "all" && args[1] == "all"
            {
                return self.part_all ();
            }

            if task_name == "all"
            {
                return self.part_all_task (&args[1..]);
            }

            if args[1] == "all"
            {
                return self.part_all_part (task_name);
            }

            let Some (task) = self.tasks.get_mut (task_name) else
            {
                return Err (format! ("Task {task_name} has not yet been added"));
            };
            for &arg in &args[1..]
            {
                let Some (participant) = self.participants.get_mut (arg) else
                {
                    continue;   // ignore it if they entered a bad name
                };
                // if this participant is paying for this task, don't add it to their list of tasks
                //if participant.paid_tasks.contains (task_name)
                //{
                //    return Ok (());
                //}
                // done with preparing, add stuff together
                participant.tasks.insert (String::from (task_name));
                participant.sum = None;
                task.participants.insert (String::from (arg));
            }
            Ok (())
        }

        /// used to add all participants to all tasks
        fn part_all (&mut self) -> PaymentResult
        {
            for task in self.tasks.values_mut ()
            {
                for part in self.participants.values_mut ()
                {
                    part.tasks.insert (task.name.clone ());
                    part.sum = None;
                    task.participants.insert (part.name.clone ());
                }
            }
            Ok (())
        }

        fn part_all_task (&mut self, args: &[&str]) -> PaymentResult
        {
            for task in self.tasks.values_mut ()
            {
                for &arg in args
                {
                    let Some (participant) = self.participants.get_mut (arg) else
                    {
                        continue;
                    };
                    participant.tasks.insert (task.name.clone ());
                    participant.sum = None;
                    task.participants.insert (String::from (arg));
                }
            }
            Ok (())
        }

        fn part_all_part (&mut self, task_name: &str) -> PaymentResult
        {
            let Some (task) = self.tasks.get_mut (task_name) else
            {
                return Err (format! ("Task {task_name} has not yet been added"));
            };
            for part in self.participants.values_mut ()
            {
                part.tasks.insert (task.name.clone ());
                part.sum = None;
                task.participants.insert (part.name.clone ());
            }
            Ok (())
        }

        fn payment (&mut self, args: &[&str]) -> PaymentResult
        {
            let amount = Self::verify_amount (args.get (2))?;
            self.verify_part (args.get(0))?;
            self.verify_part (args.get(1))?;
            {
                let participant = self.participants.get_mut (args[0]).unwrap ();
                if let Some (fu) = &mut participant.payments_made
                {
                    fu.push (amount);
                }
                else
                {
                    participant.payments_made = Some (vec![amount]);
                }
            }
            let recipient = self.participants.get_mut (args[1]).unwrap ();
            if let Some (fu) = &mut recipient.payments_made
            {
                fu.push (-amount);
            }
            else
            {
                recipient.payments_made = Some (vec![-amount]);
            }
            Ok (())
        }

        fn verify_part (&self, arg: Option<&&str>) -> PaymentResult
        {
            let part_name = match arg
            {
                Some (&"") => return Err (String::from ("Not enough arguments")),
                Some (&n) => n,
                None => return Err (String::from ("Not enough arguments")),
            };
            if !self.participants.contains_key (part_name)
            {
                return Err (format! ("Participant {} has not yet been added", part_name));
            }
            Ok (())
        }

        fn verify_amount (arg: Option<&&str>) -> Result<i32, String>
        {
            let price_string = match arg
            {
                Some (&"") => return Err (String::from ("Not enough arguments")),
                Some (&n) => n,
                None => return Err (String::from ("Not enough arguments")),
            };
            match price_string.parse::<f32> ()
            {
                Ok (p) => Ok ((p * 100f32) as i32),
                Err (_) => return Err (format! (
                        "{} not a valid decimal number for the price"
                        , price_string)),
            }
        }
    }
}
